#!/usr/bin/env python3
"""Qualify live GPU work counters and scoped streamed-world HUD glyphs."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display
from qualify_world_timings import qualify as qualify_timings
from world_timing_reference import verify
from world_camera_reference import require


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    report=dict(passed=False,captures=[],hud=True);result=out/'report.json';result.write_text(json.dumps(report)+'\n')
    scene=json.loads(args.scene.read_bytes());varied=dict(scene,quadrants=json.loads(args.variant.read_bytes())['quadrants'])
    variant=out/'varied.json';variant.write_text(json.dumps(varied))
    try:
        base=argparse.Namespace(**vars(args));base.hud=True;base.output=out/'base'
        qualify_timings(base);report['base']=json.loads((base.output/'report.json').read_bytes())
        caps=[400000,840704,600000];timing=out/'variant-timings.json';camera=out/'variant-camera.json'
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            with (out/'variant.log').open('w') as log:
                proc=subprocess.run([str(args.renderer.resolve()),'--world-camera-test',str(variant),'--world-camera-crowd','10000','--world-camera-timings',str(timing),'--world-camera-report',str(camera),'--world-camera-hud','--world-camera-warmup','10','--frames','240','--bundle-byte-budget',str(caps[0]),'--bundle-gpu-budget',str(caps[1]),'--texture-budget',str(caps[2]),'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT,timeout=90)
            require(proc.returncode==0 and 'Validation totals (including teardown): 0 warnings, 0 errors' in (out/'variant.log').read_text(),'Varied HUD native run failed/unclean')
            report['variant_native']=verify(json.loads(timing.read_bytes()),json.loads(camera.read_bytes()),varied,args.renderer,240,240,10,10000,caps,True)
            if not args.native_only:
                plans=[('variant130',variant,240,130,caps,None),('coarse130',args.scene,240,130,[142040,732544,524288],None),
                    ('shared180',args.scene,240,180,[237988,732544,524288],args.shared_shaders),
                    ('pending221',args.scene,5,221,[237988,732544,524288],None),('initial0',args.scene,240,0,[237988,732544,524288],None)]
                for name,path,period,frame,limits,shaders in plans:
                    output=out/('capture-'+name);output.mkdir(exist_ok=True)
                    env=replay.copy();env.update(THEIA_WORLD_SCENE_OUTPUT=str(output),THEIA_WORLD_SCENE=str(path.resolve()),THEIA_WORLD_SCENE_CAMERA='1',
                        THEIA_WORLD_SCENE_CROWD='10000',THEIA_WORLD_SCENE_TIMINGS='1',THEIA_WORLD_SCENE_HUD='1',THEIA_WORLD_SCENE_WARMUP='10',
                        THEIA_WORLD_SCENE_FRAMES='240',THEIA_WORLD_SCENE_PERIOD=str(period),THEIA_WORLD_SCENE_FRAME=str(frame),THEIA_WORLD_SCENE_CAPS=json.dumps(limits))
                    if shaders:env['THEIA_WORLD_SCENE_SHADERS']=str(shaders.resolve())
                    with (output/'capture.log').open('w') as log:
                        subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_scene.py'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
                    proof=json.loads((output/'report.json').read_bytes());require(proof['passed'] and proof['overlay_verified'] and len(proof['overlay_lines'])==11,'HUD replay incomplete')
                    if name=='variant130':require(proof['camera']['history'] and any(s['world']['selected_images']==14 for s in json.loads((output/'timings.json').read_bytes())['samples']),'Varied selected texture count not exercised')
                    if name=='pending221':require(proof['camera_state']['pending'] and proof['camera_state']['pending']['pending'],'Pending GPU allocation HUD window missing')
                    if name=='initial0':require(proof['overlay_lines'][1:4]==['FPS 0.0','CPU MS 0.000','GPU MS 0.000'],'First-frame unset timing defaults differ')
                    report['captures'].append(dict(case=name,**proof));print('Verified streamed HUD replay:',name,flush=True)
        report.update(passed=True,replay_qualified=not args.native_only,gpu_sizing_readback=False)
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','variant','shared-shaders','output'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    parser.add_argument('--native-only',action='store_true')
    qualify(parser.parse_args())
