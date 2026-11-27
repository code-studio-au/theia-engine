#!/usr/bin/env python3
"""Qualify two resident geometry chunks with shared visibility/depth, no streaming."""
import argparse
import json
import os
from pathlib import Path
import subprocess
from headless_display import owned_display


def require(value, message):
    if not value:
        raise RuntimeError(message)


def qualify(args):
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    report = dict(passed=False, runtime=[], captures=[])
    result_path = out / 'report.json'
    result_path.write_text(json.dumps(report)+'\n')
    scene = json.loads(args.scene.read_bytes())
    meshes = [q['cook']['path'] for q in scene['quadrants']]
    exe = str(args.renderer.resolve())
    omitted = out / 'without-world-shaders'
    omitted.mkdir(exist_ok=True)
    for path in Path('build/shaders').glob('*.spv'):
        if not path.name.startswith('world.pair.'):
            target = omitted / path.name
            if not target.exists():
                target.symlink_to(path.resolve())
    cases = [
        ('pair-recovery', ['--world-pair-test',meshes[0],meshes[2],'--wsi-smoke-test'],True),
        ('right-pair', ['--world-pair-test',meshes[1],meshes[3],'--frames','120'],True),
        ('missing-second', ['--world-pair-test',meshes[0],str(out/'missing.theia-mesh'),'--frames','3'],False),
        ('missing-world-shaders', ['--world-pair-test',meshes[0],meshes[2],'--shader-dir',str(omitted),'--frames','3'],False),
        ('original-without-world-shaders', ['--asset',meshes[0],'--material-test','--shader-dir',str(omitted),'--frames','3'],True),
        ('conflicting-camera', ['--camera-test','--world-pair-test',meshes[0],meshes[2]],False),
        ('conflicting-authored', ['--world-pair-test',meshes[0],meshes[2],'--materials',scene['quadrants'][0]['cook']['materials']['path']],False),
        ('conflicting-benchmark', ['--world-pair-test',meshes[0],meshes[2],'--benchmark'],False),
    ]
    if args.shared_shaders:
        cases.append(('shared-interface-pair', ['--world-pair-test',meshes[0],meshes[2],
            '--shader-dir',str(args.shared_shaders.resolve()),'--frames','120'],True))
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display'] = display
            for name, command, succeeds in cases:
                log_path = out/(name+'.log')
                with log_path.open('w') as log:
                    process = subprocess.run([exe,*command,'--gpu','7900'],env=runtime,
                        stdout=log,stderr=subprocess.STDOUT,timeout=60)
                text = log_path.read_text()
                require((process.returncode==0)==succeeds,name+' unexpected exit')
                if name.startswith('conflicting-'):
                    require('Selected GPU:' not in text,'conflict reached Vulkan')
                else:
                    require('Validation totals (including teardown): 0 warnings, 0 errors' in text,
                            name+' diagnostics or missing teardown')
                if succeeds:
                    require('Presented ' in text,name+' never presented')
                report['runtime'].append(dict(case=name,exit=process.returncode))
                print('Verified native world pair:',name,flush=True)
            occlusion = [0,0]
            captures = [([0,2],2,None),([2,0],2,None),([1,3],1,None),([0,2],0,None)]
            if args.shared_shaders:
                captures.append(([0,2],2,args.shared_shaders.resolve()))
            for pair,view,shaders in captures:
                output = out/(('capture-%d-%d-view%d'%(*pair,view))+('-shared' if shaders else ''))
                output.mkdir(exist_ok=True)
                child = replay.copy()
                child.update(THEIA_WORLD_PAIR_OUTPUT=str(output),
                    THEIA_WORLD_PAIR_SCENE=str(args.scene.resolve()),
                    THEIA_WORLD_PAIR_CHUNKS=json.dumps(pair),THEIA_WORLD_PAIR_VIEW=str(view))
                if shaders:
                    child['THEIA_WORLD_PAIR_SHADERS'] = str(shaders)
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_pair.py'],
                        env=child,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
                capture = json.loads((output/'report.json').read_bytes())
                require(capture['passed'],'world pair replay failed')
                occlusion = [a+b for a,b in zip(occlusion,capture['occlusion_probes'])]
                report['captures'].append(capture)
                print('Verified world pair replay:',pair,view,flush=True)
            require(all(occlusion),'both identity namespaces must win occlusion probes')
            report['occlusion_probes'] = occlusion
            report['passed'] = True
    finally:
        result_path.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--shared-shaders',type=Path)
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
