"""Owned failure/replay cases shared by the camera-world qualification runner."""
import json
from pathlib import Path
import shutil
import subprocess
import time
from world_camera_reference import require


def failure_cases(args,out,runtime,scene):
    results=[]
    for name in ('source-failure','generation-failure'):
        path=args.scene.resolve();extra=[];target=None
        if name=='source-failure':
            fixture=json.loads(json.dumps(scene));directory=out/'private-source';directory.mkdir(exist_ok=True)
            cook=fixture['quadrants'][0]['cook']
            for record in (cook['bundle'],cook,cook['materials'],cook['uv'],cook['texture_table'],*cook['textures']):
                original=Path(record['path']);copy=directory/original.name;shutil.copyfile(original,copy);record['path']=str(copy)
            path=out/'private-scene.json';path.write_text(json.dumps(fixture));target=Path(cook['bundle']['path'])
        else:
            directory=out/'failure-shaders';directory.mkdir(exist_ok=True)
            for shader in Path('build/shaders').glob('*.spv'):
                link=directory/shader.name
                if not link.exists():link.symlink_to(shader.resolve())
            target=directory/'world.scene.resolve.frag.spv';extra=['--shader-dir',str(directory)]
        log_path=out/(name+'.log');history=out/(name+'-history.json')
        with log_path.open('w') as log:
            child=subprocess.Popen([str(args.renderer.resolve()),'--world-camera-test',str(path),'--frames','240',
                '--world-camera-report',str(history),*extra,'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT)
            try:
                deadline=time.monotonic()+45
                while child.poll() is None and time.monotonic()<deadline:
                    try:data=json.loads(history.read_bytes())
                    except (FileNotFoundError,json.JSONDecodeError):time.sleep(.005);continue
                    ready=(data['evictions']>0 and data['final']['selected'][0]==4) if name=='source-failure' else data['publications']>0
                    if ready:
                        if name=='source-failure':
                            blob=bytearray(target.read_bytes());blob[-1]^=1;target.write_bytes(blob)
                        else:target.unlink()
                        break
                    time.sleep(.005)
                else:raise RuntimeError('Failed to reach controlled camera failure')
                code=child.wait(timeout=45)
            finally:
                if child.poll() is None:child.kill();child.wait()
        text=log_path.read_text();data=json.loads(history.read_bytes())
        require(code!=0 and 'Validation totals (including teardown): 0 warnings, 0 errors' in text and
            not data['completed'] and data['final']['protected_epochs']==[1]*4 and
            data['final']['epoch']==1+data['publications']+data['evictions'],name+' cleanup/publication differs')
        require(('source failed' if name=='source-failure' else 'world.scene.resolve.frag.spv') in text,'Wrong camera failure')
        if name=='source-failure':require(data['final']['selected'][0]==4,'Invalid source falsely published')
        results.append(dict(case=name,exit=code,incomplete_report=True));print('Verified camera failure:',name,flush=True)
    return results


def replay_cases(args,out,replay,variant,base_oracle,var_oracle):
    full=[237988,691840,524288];var_caps=[400000,800000,600000]
    plans=[('route110',args.scene,240,2,full,110,None),('route180',args.scene,240,2,full,180,None),
        ('variant130',variant,240,2,var_caps,130,None),('shared180',args.scene,240,2,full,180,args.shared_shaders),
        ('coarse-cpu',args.scene,240,2,[142040,full[1],full[2]],130,None),
        ('coarse-gpu',args.scene,240,2,[full[0],334368,full[2]],130,None),
        ('coarse-images',args.scene,240,2,[full[0],full[1],262144],130,None),
        ('ready-cancellation',args.scene,5,2,full,224,None),('pending-upload',args.scene,5,2,full,221,None)]
    reports=[]
    for name,path,period,slots,caps,frame,shaders in plans:
        output=out/('capture-'+name);output.mkdir(exist_ok=True)
        env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(path.resolve()),
            THEIA_WORLD_SCENE_CAMERA='1',THEIA_WORLD_SCENE_FRAME=str(frame),THEIA_WORLD_SCENE_FRAMES='240',
            THEIA_WORLD_SCENE_PERIOD=str(period),THEIA_WORLD_SCENE_SLOTS=str(slots),THEIA_WORLD_SCENE_CAPS=json.dumps(caps))
        if shaders:env['THEIA_WORLD_SCENE_SHADERS']=str(shaders.resolve())
        with (output/'capture.log').open('w') as log:
            subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],env=env,
                stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
        capture=json.loads((output/'report.json').read_bytes());require(capture['passed'],'Camera replay failed')
        (var_oracle if path==variant else base_oracle).verify(capture['camera'],240,caps)
        if name=='pending-upload':require(capture['camera_state']['pending'] and capture['camera_state']['pending']['pending'],
            'Captured pending GPU window was not exercised')
        if name=='ready-cancellation':require(capture['camera']['ready_cancellations']>0,'Ready cancellation not exercised in replay')
        reports.append(dict(case=name,**capture));print('Verified camera world replay:',name,flush=True)
    return reports
