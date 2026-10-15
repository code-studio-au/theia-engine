#!/usr/bin/env python3
"""Qualify camera-streamed cooked quadrants with up to 10000 GPU-grounded proxies."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display
from world_camera_reference import Oracle,require
from world_camera_qualification import failure_cases

FIXED=40704


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    report=dict(passed=False,native=[],captures=[],timed_benchmark=False);result=out/'report.json'
    result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes());varied=dict(scene,quadrants=json.loads(args.variant.read_bytes())['quadrants'])
    variant=out/'varied.json';variant.write_text(json.dumps(varied))
    base=Oracle(scene,args.renderer);var=Oracle(varied,args.renderer)
    full=[237988,691840+FIXED,524288];one=[166164,423888+FIXED,327680];var_caps=[400000,800000+FIXED,600000]
    def cap_flags(caps):return sum(([flag,str(value)] for flag,value in zip(('--bundle-byte-budget','--bundle-gpu-budget','--texture-budget'),caps)),[])
    def check(data,path,frames,caps):
        oracle=var if path==variant else base
        proof=oracle.verify(data,frames,[caps[0],caps[1]-FIXED,caps[2]])
        require(data['fixed_gpu_bytes']==FIXED and data['peak_total_gpu_bytes']==data['peak_gpu_bytes']+FIXED and data['peak_total_gpu_bytes']<=caps[1],'Persistent query table charge differs/exceeds cap')
        return proof
    plans=[('full',args.scene,10000,240,240,2,full,[]),('zero',args.scene,0,240,240,2,full,[]),
        ('single',args.scene,1,240,240,2,full,[]),('thousand',args.scene,1000,240,240,2,full,[]),
        ('one-fine',args.scene,10000,240,240,1,one,[]),('four-fine',args.scene,10000,240,240,4,full,[]),
        ('variant',variant,10000,240,240,2,var_caps,[]),('shared',args.scene,10000,240,240,2,full,['--shader-dir',str(args.shared_shaders.resolve())]),
        ('coarse-cpu',args.scene,10000,240,240,2,[142040,full[1],full[2]],[]),
        ('coarse-gpu',args.scene,10000,240,240,2,[full[0],334368+FIXED,full[2]],[]),
        ('coarse-images',args.scene,10000,240,240,2,[full[0],full[1],262144],[]),
        ('recovery',args.scene,10000,120,120,2,full,['--wsi-smoke-test']),
        ('repeated',args.scene,10000,1320,120,2,full,[]),('fast-turns',args.scene,10000,240,5,2,full,[]),
        ('early-shutdown',args.scene,10000,3,240,2,full,[])]
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            for name,path,players,frames,period,slots,caps,extra in plans:
                history=out/(name+'-history.json');log_path=out/(name+'.log')
                flags=['--world-camera-test',str(path.resolve()),'--world-camera-crowd',str(players),
                    '--world-camera-period',str(period),'--world-camera-slots',str(slots),
                    '--world-camera-report',str(history),*cap_flags(caps),*extra]
                if name!='recovery':flags+=['--frames',str(frames)]
                with log_path.open('w') as log:
                    proc=subprocess.run([str(args.renderer.resolve()),*flags,'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text();require(proc.returncode==0 and 'VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text and 'Presented %d frames'%frames in text,name+' incomplete/unclean')
                data=json.loads(history.read_bytes());proof=check(data,path,frames,caps)
                if name.startswith('coarse-'):require(data['publications']==0 and data['requests']<=4,'Capacity deferral churned')
                if name=='repeated':require(data['dropped_history']>0 and len(data['history'])==64 and data['publications']>20,'Repeated history/lifetime gate missing')
                if name=='fast-turns':require(data['ready_cancellations']>0,'Obsolete ready cancellation missing')
                if name=='recovery':require('Render Lab teardown: %d swapchain generations'%(7+data['publications']+data['evictions']) in text,'Recovery generations differ')
                report['native'].append(dict(case=name,players=players,proof=proof,data=data));print('Verified streamed crowd:',name,flush=True)
            for name,caps in [('short-coarse', [full[0],334368+FIXED-1,full[2]]),('short-table',[full[0],FIXED-1,full[2]])]:
                history=out/(name+'-history.json');log_path=out/(name+'.log')
                with log_path.open('w') as log:
                    proc=subprocess.run([str(args.renderer.resolve()),'--world-camera-test',str(args.scene.resolve()),'--world-camera-crowd','10000',*cap_flags(caps),'--world-camera-report',str(history),'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=45)
                require(proc.returncode!=0 and 'Validation totals (including teardown): 0 warnings, 0 errors' in log_path.read_text() and not history.exists(),'Bootstrap cap failure/cleanup differs')
                report['native'].append(dict(case=name,exit=proc.returncode))
            for name,flags in [('no-camera',['--world-camera-crowd','10000']),('bad-count',['--world-camera-test',str(args.scene.resolve()),'--world-camera-crowd','10001']),
                ('conflict-benchmark',['--world-camera-test',str(args.scene.resolve()),'--world-camera-crowd','10000','--benchmark']),
                ('conflict-retire',['--world-camera-test',str(args.scene.resolve()),'--world-camera-crowd','10000','--world-covered-retire-test'])]:
                proc=subprocess.run([str(args.renderer.resolve()),*flags],env=runtime,capture_output=True,text=True,timeout=10)
                require(proc.returncode!=0 and 'Selected GPU:' not in proc.stdout,'Conflict reached Vulkan');report['native'].append(dict(case=name,exit=proc.returncode))
            args.crowd=10000
            report['native']+=failure_cases(args,out,runtime,scene)
            if not args.native_only:
                captures=[('route110',args.scene,10000,240,110,full,None),('route180',args.scene,10000,240,180,full,None),
                    ('variant130',variant,10000,240,130,var_caps,None),('shared180',args.scene,10000,240,180,full,args.shared_shaders),
                    ('coarse130',args.scene,10000,240,130,[142040,full[1],full[2]],None),
                    ('fast224',args.scene,10000,5,224,full,None),('pending221',args.scene,10000,5,221,full,None),
                    ('zero110',args.scene,0,240,110,full,None)]
                for name,path,players,period,frame,caps,shaders in captures:
                    output=out/('capture-'+name);output.mkdir(exist_ok=True)
                    env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(path.resolve()),THEIA_WORLD_SCENE_CAMERA='1',THEIA_WORLD_SCENE_CROWD=str(players),THEIA_WORLD_SCENE_FRAMES='240',THEIA_WORLD_SCENE_FRAME=str(frame),THEIA_WORLD_SCENE_PERIOD=str(period),THEIA_WORLD_SCENE_CAPS=json.dumps(caps))
                    if shaders:env['THEIA_WORLD_SCENE_SHADERS']=str(shaders.resolve())
                    with (output/'capture.log').open('w') as log:
                        subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                    proof=json.loads((output/'report.json').read_bytes());require(proof['passed'],'Streamed crowd replay failed');check(proof['camera'],path,240,caps)
                    require(proof['crowd']['players']==players and (proof['proxy_pixels']>0 if players else proof['proxy_pixels']==0),'Missing/unexpected proxy pixels')
                    if name=='pending221':require(proof['camera_state']['pending'] and proof['camera_state']['pending']['pending'],'Ready-pending window missing')
                    if name=='fast224':require(proof['camera']['ready_cancellations']>0,'Ready cancellation not observed')
                    report['captures'].append(dict(case=name,**proof));print('Verified streamed crowd replay:',name,flush=True)
                require(all(sum(c['cross_occlusion_probes'][i] for c in report['captures'])>0 for i in (0,1)),'Two-way world/proxy occlusion missing')
            report.update(passed=True,replay_qualified=not args.native_only,authority=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','variant','output','shared-shaders'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    parser.add_argument('--native-only',action='store_true')
    qualify(parser.parse_args())
