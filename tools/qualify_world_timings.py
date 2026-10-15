#!/usr/bin/env python3
"""Verify finite streamed-world timing reports, snapshots, clocks and legacy scope."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display
from world_camera_reference import require
from world_camera_qualification import failure_cases
from world_timing_reference import verify


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';report=dict(passed=False,native=[],captures=[],hud=False)
    result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes());exe=str(args.renderer.resolve())
    full=[237988,732544,524288];one=[166164,464592,327680];coarse=[142040,732544,524288]
    def caps_flags(caps):return sum(([flag,str(value)] for flag,value in zip(('--bundle-byte-budget','--bundle-gpu-budget','--texture-budget'),caps)),[])
    plans=[('full',10000,240,240,10,2,full,[]),('zero',0,240,240,10,2,full,[]),
        ('one-fine',10000,240,240,10,1,one,[]),('coarse',10000,240,240,10,2,coarse,[]),
        ('shared',10000,240,240,10,2,full,['--shader-dir',str(args.shared_shaders.resolve())]),
        ('repeated',10000,360,120,30,2,full,[]),('fast-turns',10000,240,5,10,2,full,[]),
        ('early',10000,3,240,0,2,full,[])]
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            for name,players,frames,period,warmup,slots,caps,extra in plans:
                timing=out/(name+'-timings.json');camera=out/(name+'-camera.json');log_path=out/(name+'.log')
                command=['--world-camera-test',str(args.scene.resolve()),'--world-camera-crowd',str(players),
                    '--frames',str(frames),'--world-camera-period',str(period),'--world-camera-slots',str(slots),
                    '--world-camera-report',str(camera),'--world-camera-timings',str(timing),'--world-camera-warmup',str(warmup),*caps_flags(caps),*extra]
                with log_path.open('w') as log:
                    proc=subprocess.run([exe,*command,'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text();require(proc.returncode==0 and 'VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text and 'Presented %d frames'%frames in text,name+' incomplete/unclean native run')
                proof=verify(json.loads(timing.read_bytes()),json.loads(camera.read_bytes()),scene,args.renderer,frames,period,warmup,players,caps)
                report['native'].append(dict(case=name,proof=proof));print('Verified streamed world timings:',name,flush=True)
            # Fatal bootstrap must leave no completed timing file or false camera publication.
            timing=out/'failed-bootstrap.json';camera=out/'failed-bootstrap-camera.json';log_path=out/'failed-bootstrap.log'
            with log_path.open('w') as log:
                proc=subprocess.run([exe,'--world-camera-test',str(args.scene.resolve()),'--world-camera-crowd','10000',
                    '--world-camera-timings',str(timing),'--world-camera-report',str(camera),*caps_flags([full[0],375071,full[2]]),'--gpu','7900'],
                    env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=45)
            require(proc.returncode!=0 and not timing.exists() and not camera.exists() and
                'Validation totals (including teardown): 0 warnings, 0 errors' in log_path.read_text(),'Failed bootstrap produced report/leaked resources')
            report['native'].append(dict(case='failed-bootstrap',exit=proc.returncode))
            primary=['--world-camera-test',str(args.scene.resolve()),'--world-camera-crowd','10000']
            for name,flags in [('no-camera',['--world-camera-timings',str(out/'unused.json')]),
                ('no-crowd',['--world-camera-test',str(args.scene.resolve()),'--world-camera-timings',str(out/'unused.json')]),
                ('warmup-without-timing',[*primary,'--world-camera-warmup','10']),
                ('invalid-warmup',[*primary,'--frames','3','--world-camera-timings',str(out/'unused.json'),'--world-camera-warmup','1']),
                ('same-report',[*primary,'--world-camera-timings',str(out/'unused.json'),'--world-camera-report',str(out/'unused.json')]),
                ('recovery-incompatible',[*primary,'--world-camera-timings',str(out/'unused.json'),'--wsi-smoke-test'])]:
                proc=subprocess.run([exe,*flags],env=runtime,capture_output=True,text=True,timeout=10)
                require(proc.returncode!=0 and 'Selected GPU:' not in proc.stdout,'Invalid timing plan reached Vulkan')
                report['native'].append(dict(case=name,exit=proc.returncode))
            args.crowd=10000;failed_report=out/'fatal-timings.json';args.profile_flags=['--world-camera-timings',str(failed_report)]
            report['native']+=failure_cases(args,out,runtime,scene)
            require(not failed_report.exists(),'Fatal live source/generation failure completed timing report')
            if not args.native_only:
                for name,players,frames,period,frame,warmup in [('full',10000,240,240,110,10),('repeated',10000,360,120,280,30),('zero',0,240,240,110,10)]:
                    output=out/('capture-'+name);output.mkdir(exist_ok=True)
                    env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(args.scene.resolve()),THEIA_WORLD_SCENE_CAMERA='1',
                        THEIA_WORLD_SCENE_CROWD=str(players),THEIA_WORLD_SCENE_TIMINGS='1',THEIA_WORLD_SCENE_WARMUP=str(warmup),
                        THEIA_WORLD_SCENE_FRAMES=str(frames),THEIA_WORLD_SCENE_PERIOD=str(period),THEIA_WORLD_SCENE_FRAME=str(frame),THEIA_WORLD_SCENE_CAPS=json.dumps(full))
                    with (output/'capture.log').open('w') as log:
                        subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                    proof=json.loads((output/'report.json').read_bytes());require(proof['passed'] and proof['timing_proof']['samples']==frames,'Timed world replay failed/incomplete')
                    report['captures'].append(dict(case=name,**proof));print('Verified timed world replay:',name,flush=True)
            report.update(passed=True,replay_qualified=not args.native_only,performance_target_claim=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','shared-shaders','output'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    parser.add_argument('--native-only',action='store_true')
    qualify(parser.parse_args())
