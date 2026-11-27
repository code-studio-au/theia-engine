#!/usr/bin/env python3
"""Qualify four complete resident chunks, nonuniform resolve and bounded startup."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display


def require(value, message):
    if not value:
        raise RuntimeError(message)


def qualify(args):
    out = args.output.resolve();out.mkdir(parents=True, exist_ok=True)
    report = dict(passed=False, runtime=[], captures=[])
    result = out/'report.json';result.write_text(json.dumps(report)+'\n')
    omitted = out/'without-scene-shaders';omitted.mkdir(exist_ok=True)
    for path in Path('build/shaders').glob('*.spv'):
        if not path.name.startswith('world.scene.'):
            target = omitted/path.name
            if not target.exists():target.symlink_to(path.resolve())
    scene = json.loads(args.scene.read_bytes())
    invalid = out/'three-chunks.json'
    invalid.write_text(json.dumps(dict(scene,quadrants=scene['quadrants'][:3])))
    base = ['--world-scene-test',str(args.scene.resolve())]
    cases = [
        ('recovery',base+['--wsi-smoke-test'],True),
        ('variant',['--world-scene-test',str(args.variant.resolve()),'--frames','120'],True),
        ('empty',base+['--work-items','0','--frames','3'],True),
        ('shared',base+['--shader-dir',str(args.shared_shaders.resolve()),'--frames','120'],True),
        ('missing-module',base+['--shader-dir',str(omitted),'--frames','3'],False),
        ('three-chunks',['--world-scene-test',str(invalid),'--frames','3'],False),
        ('pair-without-scene',['--world-bundle-pair',scene['quadrants'][0]['cook']['bundle']['path'],
            scene['quadrants'][2]['cook']['bundle']['path'],'--shader-dir',str(omitted),'--frames','3'],True),
        ('probe-without-scene',['--world-descriptor-test','--shader-dir',str(omitted)],True),
        ('conflicting-benchmark',base+['--benchmark'],False),
        ('conflicting-asset',['--asset',scene['quadrants'][0]['cook']['path'],*base],False),
        ('conflicting-material',base+['--material-test'],False),
    ]
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display'] = display
            proof = None
            index = 0
            while index < len(cases):
                name,flags,success = cases[index];index += 1
                log_path = out/(name+'.log')
                with log_path.open('w') as log:
                    process = subprocess.run([str(args.renderer.resolve()),*flags,'--gpu','7900'],
                        env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text = log_path.read_text()
                require((process.returncode==0)==success,name+' unexpected exit')
                if name.startswith('conflicting-'):
                    require('Selected GPU:' not in text,name+' reached Vulkan')
                else:
                    require('VK_LAYER_KHRONOS_validation' in text and
                        'Validation totals (including teardown): 0 warnings, 0 errors' in text,
                        name+' missing validation/clean teardown')
                if success:require('Presented ' in text,name+' incomplete presentation')
                row = dict(case=name,exit=process.returncode)
                if name in ('recovery','variant','shared','exact'):
                    prefix = 'Resident four-chunk world ready: '
                    values = [json.loads(line[len(prefix):]) for line in text.splitlines() if line.startswith(prefix)]
                    require(len(values)==1 and values[0]['chunks']==4 and values[0]['root_buffers']==16
                        and values[0]['cpu_epoch']==1,name+' missing complete publication')
                    row['proof'] = values[0]
                    if name=='recovery':
                        require('Presented 120 frames' in text and 'Render Lab teardown: 7 swapchain generations' in text,
                            'Recovery sequence incomplete')
                        proof = values[0]
                        caps = [('cpu','--bundle-byte-budget',proof['cpu_unique_bytes']),
                            ('gpu','--bundle-gpu-budget',proof['physical_bytes']),
                            ('images','--texture-budget',proof['image_bytes'])]
                        exact = sum(([flag,str(limit)] for _,flag,limit in caps),[])
                        cases.append(('exact',base+exact+['--frames','120'],True))
                        for kind,flag,limit in caps:
                            cases.append(('short-'+kind,base+[flag,str(limit-1),'--frames','3'],False))
                if name.startswith('short-'):
                    require('Resident four-chunk world ready:' not in text and 'Presented ' not in text,
                        name+' published incomplete startup')
                report['runtime'].append(row)
                print('Verified native four-chunk case:',name,flush=True)
            materials = set();occlusion = [0]*4
            captures = [('base-full',args.scene,2,None),('base-angled',args.scene,1,None),
                ('base-empty',args.scene,0,None),('variant-full',args.variant,2,None),
                ('variant-angled',args.variant,1,None),('variant-shared',args.variant,2,args.shared_shaders)]
            for name,scene_path,view,shaders in captures:
                output = out/name;output.mkdir(exist_ok=True)
                env = replay.copy()
                env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(scene_path.resolve()),
                    THEIA_WORLD_SCENE_VIEW=str(view))
                if shaders:env['THEIA_WORLD_SCENE_SHADERS'] = str(shaders.resolve())
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],
                        env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                capture = json.loads((output/'report.json').read_bytes())
                require(capture['passed'],name+' replay failed')
                materials.update(map(tuple,capture['sampled_materials']))
                occlusion = [a+b for a,b in zip(occlusion,capture['occlusion_probes'])]
                report['captures'].append(dict(case=name,**capture))
                print('Verified four-chunk replay:',name,flush=True)
            require(materials=={(chunk,material) for chunk in range(4) for material in range(4)},
                'Missing chunk/material probes')
            report.update(passed=True,sampled_materials=sorted(materials),occlusion_probes=occlusion,
                live_replacement=False,camera_demand=False,authority=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('scene','variant','shared-shaders','output'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
