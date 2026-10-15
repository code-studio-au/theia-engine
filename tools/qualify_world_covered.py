#!/usr/bin/env python3
"""Qualify protected coarse GPU startup and fine admission under resource caps."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display


def require(value,message):
    if not value:raise RuntimeError(message)


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';report=dict(passed=False,native=[],captures=[]);result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes());variant=json.loads(args.variant.read_bytes())
    require(scene['passed'] and variant['passed'] and len(scene['fallbacks'])==4,'Incomplete protected fixture')
    coarse=out/'coarse.json';coarse.write_text(json.dumps(dict(scene,quadrants=scene['fallbacks'])))
    varied=out/'varied.json';varied.write_text(json.dumps(dict(scene,quadrants=variant['quadrants'])))
    invalid=out/'invalid.json';invalid.write_text(json.dumps(dict(scene,fallbacks=scene['fallbacks'][:3])))
    missing=out/'without-scene';missing.mkdir(exist_ok=True)
    for path in Path('build/shaders').glob('*.spv'):
        if not path.name.startswith('world.scene.'):
            target=missing/path.name
            if not target.exists():target.symlink_to(path.resolve())
    exe=str(args.renderer.resolve());base=['--world-covered-test',str(args.scene.resolve())]
    def caps(values):return sum(([flag,str(v)] for flag,v in zip(('--bundle-byte-budget','--bundle-gpu-budget','--texture-budget'),values)),[])
    def proof(text,prefix):
        values=[json.loads(line[len(prefix):]) for line in text.splitlines() if line.startswith(prefix)]
        require(len(values)==1,'Missing unique startup proof');return values[0]
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            plans=[('coarse-reference',['--world-scene-test',str(coarse),'--frames','3'],True,None),
                ('full-recovery',base+['--wsi-smoke-test'],True,[0,1,2,3])]
            full=low=None;index=0
            while index<len(plans):
                name,flags,success,selection=plans[index];index+=1
                log_path=out/(name+'.log')
                with log_path.open('w') as log:
                    process=subprocess.run([exe,*flags,'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text();require((process.returncode==0)==success,name+' unexpected exit')
                if name.startswith('conflict-'):require('Selected GPU:' not in text,name+' reached Vulkan')
                else:require('VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text,name+' missing validation/clean teardown')
                if success:require('Presented ' in text,name+' incomplete presentation')
                else:require('Presented ' not in text,name+' unexpectedly presented')
                row=dict(case=name,exit=process.returncode)
                if name=='coarse-reference':
                    p=proof(text,'Resident four-chunk world ready: ');low=[p['cpu_unique_bytes'],p['physical_bytes'],p['image_bytes']]
                elif success and selection is not None:
                    p=proof(text,'Protected four-chunk world ready: ')
                    require(p['selected']==selection and p['protected_epochs']==[1]*4,name+' selection/protection differs')
                    fine=sum(i<4 for i in selection)
                    require(p['owned_chunks']==4+fine and p['root_buffers']==(4+fine)*4 and p['cpu_epoch']==1+fine,
                        name+' incomplete graph/epoch ownership')
                    row['proof']=p
                    if name=='full-recovery':
                        require('Presented 120 frames' in text and 'Render Lab teardown: 7 swapchain generations' in text,
                            'Protected recovery incomplete')
                        full=[p['cpu_unique_bytes'],p['physical_bytes'],p['image_bytes']]
                        plans += [('exact',base+caps(full)+['--frames','120'],True,[0,1,2,3]),
                            ('variant',['--world-covered-test',str(varied),'--frames','120'],True,[0,1,2,3]),
                            ('shared',base+['--shader-dir',str(args.shared_shaders.resolve()),'--frames','120'],True,[0,1,2,3])]
                        for axis,kind in enumerate(('cpu','gpu','images')):
                            short=full.copy();short[axis]-=1
                            plans.append(('short-full-'+kind,base+caps(short)+['--frames','120'],True,[0,1,2,7]))
                            limit=full.copy();limit[axis]=low[axis]
                            plans.append(('coarse-only-'+kind,base+caps(limit)+['--frames','120'],True,[4,5,6,7]))
                            limit[axis]-=1
                            plans.append(('short-bootstrap-'+kind,base+caps(limit)+['--frames','3'],False,None))
                        plans += [('invalid-fallbacks',['--world-covered-test',str(invalid),'--frames','3'],False,None),
                            ('missing-module',base+['--shader-dir',str(missing),'--frames','3'],False,None),
                            ('conflict-benchmark',base+['--benchmark'],False,None),
                            ('conflict-original',['--world-scene-test',str(args.scene.resolve()),*base],False,None)]
                report['native'].append(row);print('Verified protected startup:',name,flush=True)
            captures=[('full',args.scene,[0,1,2,3],full,None),
                ('coarse-cpu',args.scene,[4,5,6,7],[low[0],full[1],full[2]],None),
                ('mixed-gpu',args.scene,[0,1,2,7],[full[0],full[1]-1,full[2]],None),
                ('mixed-images',args.scene,[0,1,2,7],[full[0],full[1],full[2]-1],None),
                ('variant',varied,[0,1,2,3],None,None),
                ('shared',args.scene,[0,1,2,3],full,args.shared_shaders),
                ('original-mode',args.scene,None,None,None)]
            for name,path,selection,limits,shaders in captures:
                output=out/('capture-'+name);output.mkdir(exist_ok=True)
                env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(path.resolve()))
                if selection is not None:env['THEIA_WORLD_SCENE_COVERED']=json.dumps(selection)
                if limits is not None:env['THEIA_WORLD_SCENE_CAPS']=json.dumps(limits)
                if shaders:env['THEIA_WORLD_SCENE_SHADERS']=str(shaders.resolve())
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],
                        env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                capture=json.loads((output/'report.json').read_bytes());require(capture['passed'],name+' replay failed')
                report['captures'].append(dict(case=name,**capture));print('Verified protected replay:',name,flush=True)
            report.update(passed=True,coarse_caps=low,full_caps=full,live_publication=False,camera_demand=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','variant','output','shared-shaders'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
