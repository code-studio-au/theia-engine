#!/usr/bin/env python3
"""Qualify fine retirement and reactivation of retained coarse GPU resources."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
from headless_display import owned_display


def require(value,message):
    if not value:raise RuntimeError(message)


def resources(cook):
    owned={}
    for record in (cook['bundle'],cook,cook['materials'],cook['uv'],cook['texture_table'],*cook['textures']):
        blob=Path(record['path']).read_bytes();key=hashlib.sha256(blob).hexdigest()
        require(key==record['content_id'] and len(blob)==record['bytes'],'Source identity/size differs')
        owned[key]=len(blob)
    return owned


def check_history(report,scene,frames,initial):
    sets={i+offset:resources(entry['cook']) for group,offset in [('quadrants',0),('fallbacks',4)]
        for i,entry in enumerate(scene[group])}
    require(report['completed'] and len(report['history'])<=5,'Incomplete/unbounded retirement history')
    selected=initial.copy();epoch=1+sum(i<4 for i in initial)
    expected=[(0,selected.copy(),epoch)]
    for slot in range(4):
        if initial[slot]==slot and frames>20*(slot+1):
            selected[slot]=slot+4;epoch+=1;expected.append((20*(slot+1),selected.copy(),epoch))
    require(len(expected)==len(report['history']) and report['retirements']==len(expected)-1,'Retirement event count differs')
    prior=None
    for row,(frame,selection,version) in zip(report['history'],expected):
        ids=set(range(4,8))|{i for i in selection if i<4}
        charge=sum({k:v for i in ids for k,v in sets[i].items()}.values())
        images=sum(len(scene['quadrants' if i<4 else 'fallbacks'][i%4]['cook']['textures']) for i in ids)
        require(row['frame']==frame and row['epoch']==version and row['selected']==selection and
            row['protected_epochs']==[1]*4 and row['owned_chunks']==len(ids) and
            row['cpu_unique_bytes']==charge and row['images']==images,'CPU coverage/epoch/owner ledger differs')
        require(row['image_bytes']==images*16384,'Qualified 64x64 RADV image ledger differs')
        if prior is not None:require(row['physical_bytes']<prior,'Retired fine allocation not released')
        prior=row['physical_bytes']
    final=report['final'];require(final['frame']==frames and final['selected']==selected and final['epoch']==epoch,'Final state differs')
    if frames>=81:
        require(selected==[4,5,6,7] and final['physical_bytes']==334368 and final['image_bytes']==262144,
            'Final coarse GPU ownership differs')


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';report=dict(passed=False,native=[],captures=[]);result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes());variant=json.loads(args.variant.read_bytes())
    varied=dict(scene,quadrants=variant['quadrants']);varied_path=out/'varied.json';varied_path.write_text(json.dumps(varied))
    broken_shaders=out/'failure-shaders';broken_shaders.mkdir(exist_ok=True)
    for path in Path('build/shaders').glob('*.spv'):
        target=broken_shaders/path.name
        if not target.exists():target.symlink_to(path.resolve())
    base=['--world-covered-test',str(args.scene.resolve()),'--world-covered-retire-test']
    exe=str(args.renderer.resolve())
    plans=[('full',base+['--frames','120'],120,scene,[0,1,2,3]),
        ('recovery',base+['--wsi-smoke-test'],120,scene,[0,1,2,3]),
        ('variant',['--world-covered-test',str(varied_path),'--world-covered-retire-test','--frames','120'],120,varied,[0,1,2,3]),
        ('shared',base+['--shader-dir',str(args.shared_shaders.resolve()),'--frames','120'],120,scene,[0,1,2,3]),
        ('mixed-start',base+['--bundle-gpu-budget','691839','--frames','120'],120,scene,[0,1,2,7]),
        ('coarse-start',base+['--bundle-byte-budget','142040','--frames','120'],120,scene,[4,5,6,7])]
    for frames in (20,21,80,81):plans.append(('prefix%d'%frames,base+['--frames',str(frames)],frames,scene,[0,1,2,3]))
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            for name,flags,frames,fixture,initial in plans:
                log_path=out/(name+'.log');history=out/(name+'-history.json')
                with log_path.open('w') as log:
                    p=subprocess.run([exe,*flags,'--world-covered-report',str(history),'--gpu','7900'],
                        env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text();require(p.returncode==0 and 'VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text and
                    'Presented %d frames'%frames in text,name+' incomplete/unclean native run')
                if name=='recovery':require('Render Lab teardown: 11 swapchain generations' in text,'Retirement/WSI recovery differs')
                data=json.loads(history.read_bytes());check_history(data,fixture,frames,initial)
                report['native'].append(dict(case=name,history=data));print('Verified retained coarse native:',name,flush=True)
            # Remove only an owned shader link after the first successful retirement.
            # The next generation must fail safely and leave the report incomplete.
            name='generation-failure';log_path=out/(name+'.log');history=out/(name+'-history.json')
            with log_path.open('w') as log:
                child=subprocess.Popen([exe,*base,'--shader-dir',str(broken_shaders),'--frames','120',
                    '--world-covered-report',str(history),'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT)
                try:
                    deadline=time.monotonic()+45
                    while child.poll() is None and time.monotonic()<deadline:
                        if 'Protected fine region 0 retired;' in log_path.read_text():
                            (broken_shaders/'world.scene.resolve.frag.spv').unlink();break
                        time.sleep(.005)
                    else:raise RuntimeError('Failed to reach controlled generation failure')
                    code=child.wait(timeout=45)
                finally:
                    if child.poll() is None:child.kill();child.wait()
            text=log_path.read_text();data=json.loads(history.read_bytes())
            require(code!=0 and 'world.scene.resolve.frag.spv' in text and
                'Validation totals (including teardown): 0 warnings, 0 errors' in text and
                not data['completed'] and data['retirements']==1,'Generation failure cleanup/report differs')
            report['native'].append(dict(case=name,exit=code,incomplete_report=True))
            for name,flags in [('conflict-profile',['--world-covered-retire-test']),
                    ('conflict-report',['--world-covered-test',str(args.scene.resolve()),'--world-covered-report',str(out/'unused.json')])]:
                p=subprocess.run([exe,*flags],env=runtime,capture_output=True,text=True,timeout=10)
                require(p.returncode!=0 and 'Selected GPU:' not in p.stdout,name+' reached Vulkan')
                report['native'].append(dict(case=name,exit=p.returncode))
            captures=[('slot0',args.scene,30,[4,1,2,3],None),('slot1',args.scene,50,[4,5,2,3],None),
                ('slot2',args.scene,70,[4,5,6,3],None),('all-coarse',args.scene,88,[4,5,6,7],None),
                ('variant-slot0',varied_path,30,[4,1,2,3],None),('variant-slot2',varied_path,70,[4,5,6,3],None),
                ('shared-all-coarse',args.scene,88,[4,5,6,7],args.shared_shaders)]
            for name,path,frame,selected,shaders in captures:
                output=out/('capture-'+name);output.mkdir(exist_ok=True)
                env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(path.resolve()),
                    THEIA_WORLD_SCENE_RETIRE='1',THEIA_WORLD_SCENE_FRAME=str(frame),THEIA_WORLD_SCENE_COVERED=json.dumps(selected))
                if shaders:env['THEIA_WORLD_SCENE_SHADERS']=str(shaders.resolve())
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],env=env,
                        stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                capture=json.loads((output/'report.json').read_bytes());require(capture['passed'],'Retirement replay failed')
                check_history(capture['retirement'],varied if path==varied_path else scene,90,[0,1,2,3])
                report['captures'].append(dict(case=name,**capture));print('Verified retained coarse replay:',name,flush=True)
            report.update(passed=True,retained_coarse_reactivation=True,camera_demand=False,fine_reload=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','variant','output','shared-shaders'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
