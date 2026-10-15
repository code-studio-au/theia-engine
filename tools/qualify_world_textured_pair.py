#!/usr/bin/env python3
"""Native/replay proof of two complete resident textured chunks and physical caps."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from headless_display import owned_display


def require(value,message):
    if not value:
        raise RuntimeError(message)


def cpu_bytes(entries):
    owned={}
    for entry in entries:
        cook=entry['cook']
        for record in [cook['bundle'],cook,cook['materials'],cook['uv'],cook['texture_table'],*cook['textures']]:
            blob=Path(record['path']).read_bytes()
            digest=hashlib.sha256(blob).hexdigest()
            require(digest==record['content_id'],'Fixture identity changed')
            owned[digest]=len(blob)
    return sum(owned.values())


def qualify(args):
    out=args.output.resolve()
    out.mkdir(parents=True,exist_ok=True)
    report=dict(passed=False,runtime=[],captures=[])
    path=out/'report.json'
    path.write_text(json.dumps(report)+'\n')
    original=json.loads(args.scene.read_bytes())
    variant=json.loads(args.variant_scene.read_bytes())
    require(original['passed'] and variant['passed'] and variant['palette_variant']['native_texels_verified'],
            'Needs checked base and independently checked palette variant')
    base=[original['quadrants'][i]['cook']['bundle']['path'] for i in (0,2)]
    pair=[variant['quadrants'][i]['cook']['bundle']['path'] for i in (0,2)]
    encoded=cpu_bytes([variant['quadrants'][i] for i in (0,2)])
    omit=out/'without-textured-pair'
    omit.mkdir(exist_ok=True)
    for shader in Path('build/shaders').glob('*.spv'):
        if shader.name!='world.pair.textured.resolve.frag.spv':
            target=omit/shader.name
            if not target.exists():
                target.symlink_to(shader.resolve())
    exe=str(args.renderer.resolve())
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (runtime,replay,display):
            report['display']=display
            def run(name,command,succeeds=True,preflight=False):
                log_path=out/(name+'.log')
                with log_path.open('w') as log:
                    process=subprocess.run([exe,*command,'--gpu','7900'],env=runtime,
                        stdout=log,stderr=subprocess.STDOUT,timeout=60)
                text=log_path.read_text()
                require((process.returncode==0)==succeeds,name+' unexpected exit')
                if preflight:
                    require('Selected GPU:' not in text,'Invalid plan reached Vulkan')
                else:
                    require('Validation totals (including teardown): 0 warnings, 0 errors' in text,
                            name+' diagnostics/missing teardown')
                if succeeds:
                    require('Presented ' in text,name+' no presentation')
                else:
                    require('Presented ' not in text,name+' failed after presenting')
                row=dict(case=name,exit=process.returncode)
                prefix='Resident textured pair ownership: '
                proofs=[json.loads(line[len(prefix):]) for line in text.splitlines() if line.startswith(prefix)]
                if proofs:
                    require(len(proofs)==1,'Duplicate owner proof')
                    row['ownership']=proofs[0]
                    ledger=re.search(r'Resident pair renderer allocation ledger: (\d+)',text)
                    require(ledger and int(ledger[1])==proofs[0]['physical_bytes'],'Renderer owner ledger mismatch')
                report['runtime'].append(row)
                print('Verified textured pair native:',name,flush=True)
                return row
            initial=run('variant-baseline',['--world-bundle-pair',*pair,'--frames','120'])['ownership']
            require(initial['cpu_unique_bytes']==encoded and initial['chunks']==2
                    and initial['root_buffers']==8 and initial['images']==8 and initial['cpu_epoch']==1,
                    'Complete owner/admission proof differs')
            physical,images=initial['physical_bytes'],initial['image_bytes']
            def caps(cpu=encoded,gpu=physical,image=images):
                return ['--bundle-byte-budget',str(cpu),'--bundle-gpu-budget',str(gpu),'--texture-budget',str(image)]
            plans=[
                ('base-recovery',['--world-bundle-pair',*base,'--wsi-smoke-test'],True,False),
                ('exact-caps',['--world-bundle-pair',*pair,'--frames','120',*caps()],True,False),
                ('short-cpu',['--world-bundle-pair',*pair,'--frames','3',*caps(cpu=encoded-1)],False,False),
                ('short-physical',['--world-bundle-pair',*pair,'--frames','3',*caps(gpu=physical-1)],False,False),
                ('short-images',['--world-bundle-pair',*pair,'--frames','3',*caps(image=images-1)],False,False),
                ('zero-physical',['--world-bundle-pair',*pair,'--frames','3',*caps(gpu=0)],False,False),
                ('missing-textured-shader',['--world-bundle-pair',*pair,'--frames','3','--shader-dir',str(omit)],False,False),
                ('geometry-without-textured-shader',['--world-pair-test',*[original['quadrants'][i]['cook']['path'] for i in (0,2)],'--frames','3','--shader-dir',str(omit)],True,False),
                ('conflicting-benchmark',['--world-bundle-pair',*pair,'--benchmark'],False,True),
                ('conflicting-stream',['--world-bundle-pair',*pair,'--stream-bundle',base[1]],False,True),
                ('shared-pair',['--world-bundle-pair',*pair,'--frames','120','--shader-dir',str(args.shared_shaders.resolve())],True,False),
            ]
            for name,command,succeeds,preflight in plans:
                run(name,command,succeeds,preflight)
            probes=[([0,2],2,True,None),([2,0],2,True,None),([1,2],1,True,None),
                    ([0,2],0,True,None),([0,2],2,True,args.shared_shaders.resolve()),
                    ([0,2],2,False,None)]
            materials=set()
            occlusion=[0,0]
            for index,(chunks,view,textured,shaders) in enumerate(probes):
                output=out/('capture-'+str(index));output.mkdir(exist_ok=True)
                env=replay.copy()
                env.update(THEIA_WORLD_PAIR_OUTPUT=str(output),THEIA_WORLD_PAIR_SCENE=str(args.variant_scene.resolve()),
                    THEIA_WORLD_PAIR_CHUNKS=json.dumps(chunks),THEIA_WORLD_PAIR_VIEW=str(view),
                    THEIA_WORLD_PAIR_TEXTURED='1' if textured else '0')
                if shaders:
                    env['THEIA_WORLD_PAIR_SHADERS']=str(shaders)
                with (output/'capture.log').open('w') as log:
                    subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_world_pair.py'],
                        env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
                capture=json.loads((output/'report.json').read_bytes())
                require(capture['passed'],'Pair replay failed')
                if textured:
                    materials.update(tuple(v) for v in capture['sampled_materials'])
                    occlusion=[a+b for a,b in zip(occlusion,capture['occlusion_probes'])]
                report['captures'].append(capture)
                print('Verified textured/geometry replay:',index,chunks,view,textured,flush=True)
            require(materials=={(chunk,material) for chunk in range(2) for material in range(4)},'Incomplete material coverage')
            require(all(occlusion),'Incomplete cross-chunk occlusion proof')
            # Preserve the original resident textured world + visual proxy/HUD path.
            output=out/'mixed-regression';output.mkdir(exist_ok=True)
            env=replay.copy();env.update(THEIA_MIXED_CAPTURE_OUTPUT=str(output),
                THEIA_MIXED_SCENE=str(args.scene.resolve()),THEIA_MIXED_PLAYERS='1000',THEIA_MIXED_OVERLAY='1')
            with (output/'capture.log').open('w') as log:
                subprocess.run([str(args.qrenderdoc.resolve()),'--python','tools/renderdoc_mixed_benchmark.py'],
                    env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
            mixed=json.loads((output/'report.json').read_bytes())
            require(mixed['passed'] and mixed['overlay_verified'],'Preserved mixed replay failed')
            report.update(passed=True,physical_bytes=physical,image_bytes=images,cpu_unique_bytes=encoded,
                          sampled_materials=sorted(materials),occlusion_probes=occlusion,mixed_regression=mixed)
    finally:
        path.write_text(json.dumps(report,indent=2)+'\n')


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
