#!/usr/bin/env python3
"""Qualify one async fine reload while protected coarse coverage stays visible."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import time
from headless_display import owned_display
from qualify_world_retirement import resources,check_history


def require(value,message):
    if not value:raise RuntimeError(message)


def check_reload(data,scene,frames,initial,outcome,caps):
    require(data['completed'] and data['fine_reload'] and len(data['history'])<=6,'Incomplete/unbounded reload run')
    status=data['reload'];require(status['region']==2,'Wrong fine reload region')
    selected=initial.copy();epoch=1+sum(i<4 for i in initial);expected=[(0,selected.copy(),epoch)]
    retirements=0
    for slot in range(4):
        if initial[slot]==slot and frames>20*(slot+1):
            selected[slot]=slot+4;epoch+=1;retirements+=1;expected.append((20*(slot+1),selected.copy(),epoch))
    if status['published']:
        require(outcome=='published','Unexpected fine reload publication')
        frame=data['history'][-1]['frame'];require(110<=frame<frames,'Fine published before hold/after completion')
        selected[2]=2;epoch+=1;expected.append((frame,selected.copy(),epoch))
    elif outcome=='published':raise RuntimeError('Expected fine reload did not publish')
    require(len(expected)==len(data['history']) and data['retirements']==retirements,'Reload event count differs')
    owned={i+offset:resources(e['cook']) for group,offset in [('quadrants',0),('fallbacks',4)] for i,e in enumerate(scene[group])}
    def charge(ids):return sum({k:v for i in ids for k,v in owned[i].items()}.values())
    def verify(row,selection,version,pending=False):
        ids=set(range(4,8))|{i for i in selection if i<4}
        images=sum(len(scene['quadrants' if i<4 else 'fallbacks'][i%4]['cook']['textures']) for i in ids)
        encoded=charge(ids|({2} if pending else set()))
        require(row['selected']==selection and row['epoch']==version and row['protected_epochs']==[1]*4 and
            row['owned_chunks']==len(ids) and row['images']==images and row['image_bytes']==images*16384 and
            row['cpu_unique_bytes']==encoded,'Published coverage/epoch/resource charge differs')
    for row,(frame,selection,version) in zip(data['history'],expected):
        require(row['frame']==frame,'Retirement event frame differs');verify(row,selection,version)
    final=data['final'];require(final['frame']==frames,'Final frame differs')
    verify(final,selected,epoch,status['pending'])
    require(final['physical_bytes']+status['pending_gpu_bytes']<=caps[1] and
        final['image_bytes']+status['pending_image_bytes']<=caps[2] and final['cpu_unique_bytes']<=caps[0],
        'Current/pending coexistence cap exceeded')
    if outcome=='cpu':require(status['cpu_deferrals']==1 and status['gpu_deferrals']==0,'CPU cap did not defer')
    elif outcome=='gpu':require(status['gpu_deferrals']==1 and status['cpu_deferrals']==0,'GPU/image cap did not defer')
    else:require(status['cpu_deferrals']==status['gpu_deferrals']==0,'Unexpected capacity failure')
    if outcome=='not-started':require(not status['started'],'Reload requested before threshold')
    elif outcome=='loading':require(status['started'] and status['loading'] and not status['pending'],'Read shutdown prefix differs')
    elif outcome=='pending':require(status['pending'] and status['pending_ready'] is True and not status['loading'],
        'Ready GPU cancellation window not observed')
    if outcome in ('published','cpu','gpu'):require(not status['loading'] and not status['pending'],'Reload left pending owners')
    if selected==[4,5,2,7]:
        target=390480 if len(scene['quadrants'][2]['cook']['textures'])==2 else 423552
        require(final['physical_bytes']==target,'Reloaded complete GPU ledger differs')
    elif selected==[4,5,6,7]:require(final['physical_bytes']==334368,'Coarse GPU owners changed')


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    report=dict(passed=False,native=[],captures=[]);result=out/'report.json';result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes());variant=json.loads(args.variant.read_bytes())
    varied=dict(scene,quadrants=variant['quadrants']);varied_path=out/'varied.json';varied_path.write_text(json.dumps(varied))
    full=[237988,691840,524288];exact=[165852,423552,327680]
    def cap_args(values):return sum(([flag,str(v)] for flag,v in zip(('--bundle-byte-budget','--bundle-gpu-budget','--texture-budget'),values)),[])
    def flags(path,frames):return ['--world-covered-test',str(path),'--world-covered-reload-test','--frames',str(frames)]
    plans=[('full',args.scene,180,full,[0,1,2,3],'published',[]),
        ('recovery',args.scene,120,full,[0,1,2,3],'published',['--wsi-smoke-test']),
        ('variant',varied_path,180,[400000,800000,600000],[0,1,2,3],'published',[]),
        ('shared',args.scene,180,full,[0,1,2,3],'published',['--shader-dir',str(args.shared_shaders.resolve())]),
        ('exact',args.scene,180,exact,[4,5,2,7],'published',[]),
        ('short-cpu',args.scene,180,[exact[0]-1,exact[1],exact[2]],[4,5,6,7],'cpu',[]),
        ('short-gpu',args.scene,180,[exact[0],exact[1]-1,exact[2]],[4,5,6,7],'gpu',[]),
        ('short-images',args.scene,180,[exact[0],exact[1],exact[2]-1],[4,5,6,7],'gpu',[]),
        ('coarse-only',args.scene,180,[142040,full[1],full[2]],[4,5,6,7],'cpu',[])]
    for frame,outcome in [(100,'not-started'),(101,'loading'),(110,'pending')]:
        plans.append(('prefix%d'%frame,args.scene,frame,full,[0,1,2,3],outcome,[]))
    exe=str(args.renderer.resolve())
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            for name,path,frames,caps,initial,outcome,extra in plans:
                command=flags(path.resolve(),frames)
                if name=='recovery':command=command[:-2] # Recovery flag owns 120 frames.
                log_path=out/(name+'.log');history=out/(name+'-history.json')
                with log_path.open('w') as log:
                    p=subprocess.run([exe,*command,*cap_args(caps),*extra,'--world-covered-report',str(history),'--gpu','7900'],
                        env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text();require(p.returncode==0 and 'VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text and
                    'Presented %d frames'%frames in text,name+' incomplete/unclean native run')
                if name=='recovery':require('Render Lab teardown: 12 swapchain generations' in text,'Reload/WSI recovery differs')
                data=json.loads(history.read_bytes());check_reload(data,varied if path==varied_path else scene,frames,initial,outcome,caps)
                report['native'].append(dict(case=name,history=data));print('Verified protected fine reload:',name,flush=True)
            # Two immutable private fixtures qualify error cleanup after coarse reactivation.
            for name in ('source-failure','generation-failure'):
                fixture=scene;extra=[];target=None
                if name=='source-failure':
                    fixture=json.loads(json.dumps(scene));directory=out/'private-source';directory.mkdir(exist_ok=True)
                    cook=fixture['quadrants'][2]['cook']
                    for record in (cook['bundle'],cook,cook['materials'],cook['uv'],cook['texture_table'],*cook['textures']):
                        original=Path(record['path']);copy=directory/original.name;shutil.copyfile(original,copy);record['path']=str(copy)
                    path=out/'private-scene.json';path.write_text(json.dumps(fixture));target=Path(cook['bundle']['path'])
                else:
                    path=args.scene.resolve();directory=out/'failure-shaders';directory.mkdir(exist_ok=True)
                    for shader in Path('build/shaders').glob('*.spv'):
                        link=directory/shader.name
                        if not link.exists():link.symlink_to(shader.resolve())
                    target=directory/'world.scene.resolve.frag.spv';extra=['--shader-dir',str(directory)]
                log_path=out/(name+'.log');history=out/(name+'-history.json')
                with log_path.open('w') as log:
                    child=subprocess.Popen([exe,*flags(path,180),*extra,'--world-covered-report',str(history),'--gpu','7900'],
                        env=runtime,stdout=log,stderr=subprocess.STDOUT)
                    try:
                        deadline=time.monotonic()+45
                        while child.poll() is None and time.monotonic()<deadline:
                            if 'Protected fine region 3 retired;' in log_path.read_text():
                                if name=='source-failure':
                                    blob=bytearray(target.read_bytes());blob[-1]^=1;target.write_bytes(blob)
                                else:target.unlink()
                                break
                            time.sleep(.005)
                        else:raise RuntimeError('Failed to reach controlled reload error')
                        code=child.wait(timeout=45)
                    finally:
                        if child.poll() is None:child.kill();child.wait()
                text=log_path.read_text();data=json.loads(history.read_bytes())
                require(code!=0 and 'Validation totals (including teardown): 0 warnings, 0 errors' in text and
                    not data['completed'] and data['retirements']==4 and len(data['history'])==5 and
                    data['final']['selected']==[4,5,6,7] and not data['reload']['published'],name+' cleanup/publication differs')
                require(('source failed' if name=='source-failure' else 'world.scene.resolve.frag.spv') in text,'Wrong reload failure')
                report['native'].append(dict(case=name,exit=code,incomplete_report=True))
            for name,command in [('conflict-no-profile',['--world-covered-reload-test']),
                ('conflict-fixed',['--world-scene-test',str(args.scene.resolve()),'--world-covered-reload-test'])]:
                p=subprocess.run([exe,*command],env=runtime,capture_output=True,text=True,timeout=10)
                require(p.returncode!=0 and 'Selected GPU:' not in p.stdout,'Conflict reached Vulkan')
                report['native'].append(dict(case=name,exit=p.returncode))
            captures=[('full',args.scene,full,[4,5,2,7],True,None),('variant',varied_path,[400000,800000,600000],[4,5,2,7],True,None),
                ('shared',args.scene,full,[4,5,2,7],True,args.shared_shaders),
                ('coarse-cpu',args.scene,[142040,full[1],full[2]],[4,5,6,7],True,None),
                ('coarse-gpu',args.scene,[exact[0],exact[1]-1,exact[2]],[4,5,6,7],True,None),
                ('coarse-images',args.scene,[exact[0],exact[1],exact[2]-1],[4,5,6,7],True,None),
                ('retirement-only',args.scene,None,[4,5,6,7],False,None)]
            for name,path,caps,selected,reload,shaders in captures:
                output=out/('capture-'+name);output.mkdir(exist_ok=True)
                env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(path.resolve()),
                    THEIA_WORLD_SCENE_COVERED=json.dumps(selected),THEIA_WORLD_SCENE_FRAME='130' if reload else '88',
                    THEIA_WORLD_SCENE_FRAMES='180' if reload else '90')
                env['THEIA_WORLD_SCENE_RELOAD' if reload else 'THEIA_WORLD_SCENE_RETIRE']='1'
                if caps:env['THEIA_WORLD_SCENE_CAPS']=json.dumps(caps)
                if shaders:env['THEIA_WORLD_SCENE_SHADERS']=str(shaders.resolve())
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],env=env,
                        stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                capture=json.loads((output/'report.json').read_bytes());require(capture['passed'],'Reload replay failed')
                if reload:
                    initial=[0,1,2,3] if name in ('full','variant','shared') else [4,5,6,7]
                    outcome='published' if name in ('full','variant','shared') else ('cpu' if name=='coarse-cpu' else 'gpu')
                    check_reload(capture['retirement'],varied if path==varied_path else scene,180,initial,outcome,caps)
                else:check_history(capture['retirement'],scene,90,[0,1,2,3])
                report['captures'].append(dict(case=name,**capture));print('Verified protected fine replay:',name,flush=True)
            report.update(passed=True,async_fine_reload=True,camera_demand=False,repeated_load_scheduler=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','variant','output','shared-shaders'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
