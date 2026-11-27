#!/usr/bin/env python3
"""Qualify camera-driven complete fine/coarse publication over protected coverage."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display
from world_camera_reference import Oracle,require
from world_camera_qualification import failure_cases,replay_cases


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';report=dict(passed=False,native=[],captures=[],replay_qualified=False);result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes());varied=dict(scene,quadrants=json.loads(args.variant.read_bytes())['quadrants'])
    variant=out/'varied.json';variant.write_text(json.dumps(varied))
    full=[237988,691840,524288];one=[166164,423888,327680];var_caps=[400000,800000,600000]
    def cap_flags(caps):return sum(([flag,str(value)] for flag,value in zip(('--bundle-byte-budget','--bundle-gpu-budget','--texture-budget'),caps)),[])
    plans=[('full',args.scene,240,240,2,full,[]),('one-fine',args.scene,240,240,1,one,[]),
        ('four-fine',args.scene,240,240,4,full,[]),('variant',variant,240,240,2,var_caps,[]),
        ('shared',args.scene,240,240,2,full,['--shader-dir',str(args.shared_shaders.resolve())]),
        ('coarse-cpu',args.scene,240,240,2,[142040,full[1],full[2]],[]),
        ('coarse-gpu',args.scene,240,240,2,[full[0],334368,full[2]],[]),
        ('coarse-images',args.scene,240,240,2,[full[0],full[1],262144],[]),
        ('recovery',args.scene,120,120,2,full,['--wsi-smoke-test']),
        ('repeated',args.scene,1320,120,2,full,[]),('fast-turns',args.scene,240,5,2,full,[]),
        ('early-shutdown',args.scene,3,240,2,full,[])]
    base_oracle=Oracle(scene,args.renderer);var_oracle=Oracle(varied,args.renderer)
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            for name,path,frames,period,slots,caps,extra in plans:
                history=out/(name+'-history.json');log_path=out/(name+'.log')
                flags=['--world-camera-test',str(path.resolve()),'--world-camera-period',str(period),
                    '--world-camera-slots',str(slots),'--world-camera-report',str(history),*cap_flags(caps),*extra]
                if name!='recovery':flags+=['--frames',str(frames)]
                with log_path.open('w') as log:
                    p=subprocess.run([str(args.renderer.resolve()),*flags,'--gpu','7900'],env=runtime,
                        stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text();require(p.returncode==0 and 'VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text and
                    'Presented %d frames'%frames in text,name+' incomplete/unclean native run')
                data=json.loads(history.read_bytes());proof=(var_oracle if path==variant else base_oracle).verify(data,frames,caps)
                if name=='recovery':require('Render Lab teardown: %d swapchain generations'%(7+data['publications']+data['evictions']) in text,'Recovery generation count differs')
                if name.startswith('coarse-'):require(data['publications']==0 and data['requests']<=4,'Capacity deferral churned/published fine content')
                if name=='repeated':require(data['dropped_history']>0 and len(data['history'])==64 and data['publications']>20,'Repeated lifetime/history gate not exercised')
                if name=='fast-turns':require(data['cancellations']>0 and data['ready_cancellations']>0,'Obsolete ready-upload cancellation not exercised')
                report['native'].append(dict(case=name,proof=proof,data=data));print('Verified camera world:',name,flush=True)
            for name,flags in [('conflict-retire',['--world-camera-test',str(args.scene.resolve()),'--world-covered-retire-test']),
                ('conflict-view',['--world-camera-test',str(args.scene.resolve()),'--work-items','2']),
                ('bad-slots',['--world-camera-test',str(args.scene.resolve()),'--world-camera-slots','0']),
                ('bad-period',['--world-camera-test',str(args.scene.resolve()),'--world-camera-period','2']),
                ('no-camera',['--world-camera-report',str(out/'unused.json')])]:
                p=subprocess.run([str(args.renderer.resolve()),*flags],env=runtime,capture_output=True,text=True,timeout=10)
                require(p.returncode!=0 and 'Selected GPU:' not in p.stdout,name+' reached Vulkan')
                report['native'].append(dict(case=name,exit=p.returncode))
            report['native']+=failure_cases(args,out,runtime,scene)
            if not args.native_only:
                report['captures']=replay_cases(args,out,replay,variant,base_oracle,var_oracle)
                report['replay_qualified']=True
            report.update(passed=True,native_only=args.native_only,camera_demand=True,authority=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','variant','output','shared-shaders'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--native-only',action='store_true')
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
