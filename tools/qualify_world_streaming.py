#!/usr/bin/env python3
"""Qualify one fixed and one independently replaced complete world chunk."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time
from headless_display import owned_display


def require(value,message):
    if not value:
        raise RuntimeError(message)


def records(cook):
    return [cook['bundle'],cook,cook['materials'],cook['uv'],cook['texture_table'],*cook['textures']]


def encoded_union(cooks):
    union={}
    for cook in cooks:
        for record in records(cook):
            data=Path(record['path']).read_bytes()
            ident=hashlib.sha256(data).hexdigest()
            require(ident==record['content_id'],'Fixture hash changed')
            union[ident]=len(data)
    return sum(union.values())


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    report=dict(passed=False,runtime=[],captures=[])
    report_path=out/'report.json';report_path.write_text(json.dumps(report)+'\n')
    base=json.loads(args.scene.read_bytes());variant=json.loads(args.variant_scene.read_bytes())
    require(base['passed'] and variant['passed'] and variant['palette_variant']['unique_images']==2,
            'Needs checked base plus two-image alias variant')
    cooks=[base['quadrants'][i]['cook'] for i in (0,2)]
    replacement=variant['quadrants'][2]['cook']
    pair=[c['bundle']['path'] for c in cooks]
    required_cpu=encoded_union([*cooks,replacement])
    current_cpu=[encoded_union(cooks),encoded_union([cooks[0],replacement])]
    exe=str(args.renderer.resolve())
    def command(name,frames=120,extra=(),initial=pair,replacement_path=replacement['bundle']['path']):
        return ['--world-bundle-pair',*initial,'--world-stream-bundle',replacement_path,
                '--world-stream-report',str(out/(name+'.json')),'--frames',str(frames),*extra]
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            def run(name,flags,preflight=False):
                log_path=out/(name+'.log')
                with log_path.open('w') as log:
                    result=subprocess.run([exe,*flags,'--gpu','7900'],env=runtime,
                        stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text()
                if preflight:
                    require(result.returncode!=0 and 'Selected GPU:' not in text,name+' reached Vulkan')
                    report['runtime'].append(dict(case=name,preflight_rejected=True))
                    return None
                require(result.returncode==0 and 'Validation totals (including teardown): 0 warnings, 0 errors' in text,
                        name+' exit/diagnostics')
                proof=json.loads((out/(name+'.json')).read_bytes())
                require(proof['fixed_epoch']==1 and all(e['fixed_epoch']==1 for e in proof['history']),
                        name+' fixed CPU chunk changed')
                # A cancelled pending set legitimately retains all three source graphs.
                expected_cpu=required_cpu if proof['pending'] else current_cpu[proof['active_index']]
                require(proof['current_cpu_bytes']==expected_cpu,name+' source ownership differs')
                require(proof['cpu_epoch']==proof['publications']+1,name+' epoch mismatch')
                report['runtime'].append(dict(case=name,proof=proof))
                print('Verified native independent world stream:',name,flush=True)
                return proof
            baseline=run('baseline',command('baseline'))
            require(baseline['publications']>=4 and baseline['deferrals']==0,'Baseline never alternated')
            require(baseline['peak_cpu_bytes']==required_cpu,'CPU coexistence union differs')
            physical=baseline['peak_complete_set_physical_bytes'];images=baseline['peak_complete_set_image_bytes']
            def caps(cpu=required_cpu,gpu=physical,image=images):
                return ['--bundle-byte-budget',str(cpu),'--bundle-gpu-budget',str(gpu),'--texture-budget',str(image)]
            for name,extra in [('exact',caps()),('shared',[*caps(),'--shader-dir',str(args.shared_shaders.resolve())])]:
                proof=run(name,command(name,extra=extra))
                require(proof['publications']>=4 and proof['deferrals']==0,name+' did not publish')
                require(proof['peak_complete_set_physical_bytes']<=physical and proof['peak_complete_set_image_bytes']<=images,
                        name+' exceeded caps')
            recovery_flags=command('recovery');index=recovery_flags.index('--frames');del recovery_flags[index:index+2]
            recovery=run('recovery',[*recovery_flags,'--wsi-smoke-test'])
            require(recovery['publications']>=2,'Recovery never published')
            deferrals=[('short-cpu',caps(cpu=required_cpu-1)),('short-physical',caps(gpu=physical-1)),
                       ('short-images',caps(image=images-1)),('resident-cpu-only',caps(cpu=current_cpu[0]))]
            for name,extra in deferrals:
                proof=run(name,command(name,extra=extra))
                require(proof['publications']==0 and proof['deferrals']>=2 and proof['active_index']==0
                        and proof['cpu_epoch']==1,name+' failed to retain both complete chunks')
            for name,frames in [('cancel-read',21),('cancel-pending',22)]:
                proof=run(name,command(name,frames=frames))
                require(proof['publications']==0 and (proof['loading'] or proof['pending']),name+' missed cancellation')
            long=run('bounded-history',command('bounded-history',frames=1320,extra=caps()))
            require(long['publications']>=64 and len(long['history'])==64 and long['dropped_history']>0,
                    'Publication history did not stay bounded')
            # Mutate only a private manifest copy after initial preflight/creation.
            # Every root/leaf is a read-only symlink; original verified assets stay intact.
            private=out/'changed-source';private.mkdir(exist_ok=True)
            private_manifest=private/Path(replacement['bundle']['path']).name
            shutil.copyfile(replacement['bundle']['path'],private_manifest)
            for record in records(replacement)[1:]:
                link=private/Path(record['path']).name
                if not link.exists():link.symlink_to(Path(record['path']).resolve())
            changed_flags=command('changed-source',replacement_path=str(private_manifest))
            log_path=out/'changed-source.log'
            with log_path.open('w') as log:
                process=subprocess.Popen([exe,*changed_flags,'--gpu','7900'],env=runtime,stdout=log,stderr=subprocess.STDOUT)
                try:
                    deadline=time.monotonic()+15
                    while 'Swapchain generation 1 ready' not in log_path.read_text() and time.monotonic()<deadline:
                        require(process.poll() is None,'Changed-source run exited before startup')
                        time.sleep(.01)
                    require('Swapchain generation 1 ready' in log_path.read_text(),'Changed-source startup timed out')
                    private_manifest.write_bytes(b'changed private manifest')
                    result=process.wait(timeout=30)
                finally:
                    if process.poll() is None:process.kill();process.wait()
            text=log_path.read_text()
            require(result!=0 and 'World chunk I/O failed' in text and 'World chunk 1 published' not in text
                    and 'Validation totals (including teardown): 0 warnings, 0 errors' in text,
                    'Changed immutable source did not fail cleanly before publication')
            report['runtime'].append(dict(case='changed-source',rejected=True))
            for name,flags in [
                ('no-pair',['--world-stream-bundle',replacement['bundle']['path']]),
                ('report-without-stream',['--world-bundle-pair',*pair,'--world-stream-report',str(out/'bad.json')]),
                ('same-replacement',['--world-bundle-pair',*pair,'--world-stream-bundle',pair[1]]),
            ]:run(name,flags,preflight=True)
            capture_plans=[(10,None,None),(30,None,None),(50,None,None),(70,None,None),
                           (30,[required_cpu,physical,images],args.shared_shaders.resolve()),
                           (30,[required_cpu,physical-1,images],None),
                           (30,[required_cpu-1,physical,images],None)]
            states=set();materials=set()
            for index,(frame,budgets,shaders) in enumerate(capture_plans):
                output=out/('capture-'+str(index));output.mkdir(exist_ok=True)
                env=replay.copy();env.update(THEIA_WORLD_PAIR_OUTPUT=str(output),
                    THEIA_WORLD_PAIR_SCENE=str(args.scene.resolve()),THEIA_WORLD_PAIR_TEXTURED='1',
                    THEIA_WORLD_PAIR_FRAME=str(frame),THEIA_WORLD_STREAM_SCENE=str(args.variant_scene.resolve()))
                if budgets:env['THEIA_WORLD_STREAM_CAPS']=json.dumps(budgets)
                if shaders:env['THEIA_WORLD_PAIR_SHADERS']=str(shaders)
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_pair.py'],env=env,
                        stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
                captured=json.loads((output/'report.json').read_bytes())
                require(captured['passed'] and captured['stream']['fixed_epoch']==1,'World stream replay failed')
                if budgets and (budgets[0]<required_cpu or budgets[1]<physical):
                    require(captured['active_index']==0 and captured['stream']['deferrals']>0,'Pressure capture published')
                elif frame==10:
                    require(captured['active_index']==0,'Initial capture already replaced')
                else:
                    require(captured['stream']['publications']>=2,'Capture never alternated')
                states.add(captured['active_index']);materials.update(tuple(m) for m in captured['sampled_materials'])
                report['captures'].append(captured)
                print('Verified world stream replay:',index,frame,captured['active_index'],flush=True)
            require(states=={0,1} and materials=={(c,m) for c in range(2) for m in range(4)},'Missing state/material proof')
            report.update(passed=True,cpu_coexistence_bytes=required_cpu,physical_coexistence_bytes=physical,
                          image_coexistence_bytes=images,authority=False,camera_driven=False)
    finally:
        report_path.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--variant-scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--shared-shaders',type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--qrenderdoc',type=Path,default=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
