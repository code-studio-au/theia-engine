#!/usr/bin/env python3
"""Draw all fine/coarse quadrant selections; no automatic runtime LOD or eviction."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display


def require(value,message):
    if not value:raise RuntimeError(message)


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    report=dict(passed=False,native=[],captures=[])
    result=out/'report.json';result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes())
    require(scene['passed'] and scene['cooked'] and len(scene['quadrants'])==len(scene['fallbacks'])==4,
        'Needs four checked fine and fallback bundles')
    try:
        with (out/'source-proof.log').open('w') as log:
            subprocess.run(['python3','tools/qualify_benchmark_scene.py','--scene',str(args.scene.resolve()),
                '--output',str(out/'source-proof')],stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
        report['source_proof']=json.loads((out/'source-proof/report.json').read_bytes())
        selections={}
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            plans=[]
            for mask in range(16):
                selected=dict(scene,quadrants=[scene['fallbacks' if mask&(1<<i) else 'quadrants'][i] for i in range(4)])
                path=out/('selection-%04d.json'%mask);path.write_text(json.dumps(selected,indent=2)+'\n')
                selections[mask]=path
                plans.append(('selection-%04d'%mask,path,['--frames','3']))
            plans.append(('coarse-recovery',selections[15],['--wsi-smoke-test']))
            for name,path,flags in plans:
                log_path=out/(name+'.log')
                with log_path.open('w') as log:
                    process=subprocess.run([str(args.renderer.resolve()),'--world-scene-test',str(path),*flags,
                        '--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text()
                require(process.returncode==0 and 'VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text,
                    name+' missing validation/clean teardown')
                require(('Presented 120 frames' if name=='coarse-recovery' else 'Presented 3 frames') in text,
                    name+' incomplete presentation')
                if name=='coarse-recovery':require('Render Lab teardown: 7 swapchain generations' in text,
                    'Coarse recovery incomplete')
                prefix='Resident four-chunk world ready: '
                proof=[json.loads(line[len(prefix):]) for line in text.splitlines() if line.startswith(prefix)]
                require(len(proof)==1 and proof[0]['chunks']==4 and proof[0]['cpu_epoch']==1,'Incomplete graph publication')
                report['native'].append(dict(case=name,proof=proof[0]))
                print('Verified native fallback selection:',name,flush=True)
            for mask,view,shared in [(15,2,False),(5,1,False),(10,2,False),(15,2,True)]:
                name='capture-mask%d-view%d%s'%(mask,view,'-shared' if shared else '')
                output=out/name;output.mkdir(exist_ok=True)
                env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),
                    THEIA_WORLD_SCENE=str(selections[mask]),THEIA_WORLD_SCENE_VIEW=str(view))
                if shared:env['THEIA_WORLD_SCENE_SHADERS']=str(args.shared_shaders.resolve())
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],
                        env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                capture=json.loads((output/'report.json').read_bytes());require(capture['passed'],'Fallback replay failed')
                report['captures'].append(dict(case=name,**capture));print('Verified fallback replay:',name,flush=True)
        report.update(passed=True,automatic_lod=False,camera_demand=False,protected_gpu_residency=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','output','shared-shaders'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
