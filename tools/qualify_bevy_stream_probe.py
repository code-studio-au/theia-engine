#!/usr/bin/env python3
"""Qualify one real cooked fine reload/retirement in the optional Bevy tool.

This controlled correctness gate is separate from the matched replay benchmark;
it neither ports camera demand nor establishes streaming/memory equivalence.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil

from export_bevy_benchmark import export
from headless_display import owned_display
from qualify_bevy_workload import verify as verify_poses
from qualify_world_retirement import resources
from verify_bevy_snapshot import verify as verify_pixels
from world_camera_reference import require

ROOT=Path(__file__).resolve().parents[1]


def check(report,fixture,expected,encoded,logical,region):
    probe=report['streaming_probe'];base=[4,5,6,7];fine=base.copy();fine[region]=region
    require(report['streaming'] and probe['phase']==expected and probe['selected']==base,'Incomplete fine lifecycle/fallback')
    require(probe['cpu_unique_bytes']==probe['base_cpu_bytes']==encoded[0] and probe['base_logical_asset_bytes']==logical[0],'Protected base charge differs')
    require(probe['physical_gpu_bytes'] is None,'Logical bytes mislabelled as physical VRAM')
    phases=[row['phase'] for row in probe['history']]
    require(phases[:2]==['coarse','loading'] and len(phases)<=8,'Unexpected/unbounded lifecycle')
    expected_phases={'retired':['coarse','loading','uploading','published','retiring','unloading','retired'],
        'deferred_cpu':['coarse','loading','deferred_cpu'],'deferred_logical':['coarse','loading','deferred_logical']}
    if expected in expected_phases:require(phases==expected_phases[expected],'Lifecycle ordering differs')
    require(('published' in phases)==(expected=='retired'),'Cancelled/deferred fine was published')
    for sample in report['samples']:
        state=sample['stream_probe'];phase=state['phase'];selected=sample['selected'];pending=phase in ('uploading','published','retiring','unloading')
        require(selected==(fine if phase=='published' else base),'Missing coarse coverage or premature selection')
        require(state['cpu_unique_bytes']==encoded[pending],'Encoded pending/reader charge differs')
        if phase=='published':require(state['gpu_ready'],'Fine published before GPU preparation/completion')
    for row in probe['history']:
        pending=row['phase'] in ('uploading','published','retiring','unloading')
        require(row['cpu_unique_bytes']==encoded[pending] and row['logical_asset_bytes']==logical[pending],'Transition charge differs')
    if expected=='retired':require(probe['watch_removal'] and probe['gpu_ready'],'Encoded fine released before GPU removal acknowledgement')
    require([s['frame'] for s in report['samples']]==list(range(len(fixture['frames']))),'Incomplete/duplicate route samples')
    return probe


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True);parser.add_argument('--source-report',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=ROOT/'build/qualification/bevy-stream-probe')
    parser.add_argument('--bevy',type=Path,default=ROOT/'tools/theia-bevy-benchmark/target/release/theia-bevy-benchmark')
    parser.add_argument('--theia',type=Path,default=ROOT/'apps/theia-render-lab/target/release/theia-render-lab')
    parser.add_argument('--headless-dri-prime');parser.add_argument('--gpu',default='7900')
    args=parser.parse_args();os.chdir(ROOT);out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    summary=dict(passed=False,scope='One-fine correctness gate, not equivalent camera streaming or a performance comparison',cases=[],negative=[])
    result=out/'report.json';result.write_text(json.dumps(summary)+'\n')
    fixture=export(args.scene,args.source_report,out/'source-fixture.json',args.theia)
    require(len(fixture['frames'])>=140,'Need at least 140 source frames')
    scene=json.loads(args.scene.read_bytes());sets=[resources(e['cook']) for group in ('quadrants','fallbacks') for e in scene[group]]
    base={k:v for graph in sets[4:] for k,v in graph.items()};encoded_base=sum(base.values())
    # Independent adapted-array charges: position + normal + UV f32 arrays,
    # each material's copied texels and two vec4 uniform fields. No allocator claim.
    logical=[sum(len(p['positions'])*32+len(p['texture']['rgba'])+32 for p in c['parts']) for c in fixture['chunks']]
    logical_base=sum(logical[4:]);region=2;encoded_fine=sum(dict(base,**sets[region]).values());logical_fine=logical_base+logical[region]
    plans=[('region-%d'%r,r,10000 if r else 0,[], 'retired') for r in range(4)]
    plans += [('exact',region,10000,['--probe-cpu-budget',str(encoded_fine),'--probe-upload-budget',str(logical_fine)],'retired'),
        ('cpu-short',region,10000,['--probe-cpu-budget',str(encoded_fine-1)],'deferred_cpu'),
        ('logical-short',region,10000,['--probe-upload-budget',str(logical_fine-1)],'deferred_logical'),
        ('coarse-only',region,0,['--probe-upload-budget',str(logical_base)],'deferred_logical'),
        ('cancel-worker',region,10000,['--probe-cancel-frame','21'],'cancelled'),
        ('cancel-ready',region,10000,['--probe-cancel-frame','30'],'cancelled')]
    with owned_display(out/'display',args.headless_dri_prime) as (env,_,display):
        env.update(WGPU_ADAPTER_NAME=args.gpu,WGPU_VALIDATION='1',WGPU_DEBUG='1',VK_INSTANCE_LAYERS='VK_LAYER_KHRONOS_validation')
        for name,region,players,flags,expected in plans:
            case=out/name;case.mkdir(exist_ok=True)
            for filename in ('report.json','pose-proof.json'):(case/filename).unlink(missing_ok=True)
            variant=dict(json.loads(json.dumps(fixture)),players=players,source_population=fixture['players'],scope='Controlled probe population/camera input; selections below are observed Bevy state, not Theia replay')
            path=case/'fixture.json';path.write_text(json.dumps(variant,separators=(',',':'))+'\n')
            command=[str(args.bevy.resolve()),'--fixture',str(path),'--output',str(case),'--pre-roll','60','--present-mode','immediate','--streaming-probe',str(region),*flags]
            print('Qualifying',name,flush=True)
            with (case/'run.log').open('w') as log:subprocess.run(command,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
            log=(case/'run.log').read_text();require(not any(s in log for s in ('VUID-','Validation Error',' ERROR ','panicked at','Falling back to')),'Native diagnostic in '+name)
            report=json.loads((case/'report.json').read_bytes());encoded=(encoded_base,sum(dict(base,**sets[region]).values()));charges=(logical_base,logical_base+logical[region])
            probe=check(report,variant,expected,encoded,charges,region)
            if name=='cancel-worker':require('uploading' not in [r['phase'] for r in probe['history']],'Worker cancellation allocated GPU assets')
            if name=='cancel-ready':require(any(s['stream_probe']['phase']=='uploading' and s['stream_probe']['gpu_ready'] for s in report['samples']),'Ready cancellation did not exercise GPU-ready pending state')
            require(report['metadata']['device_id']==json.loads(args.source_report.read_bytes())['metadata']['device_id'],'Different physical device')
            for row,sample in zip(variant['frames'],report['samples']):row['selected']=sample['selected']
            observed=case/'observed-fixture.json';observed.write_text(json.dumps(variant,separators=(',',':'))+'\n')
            poses=verify_poses(observed,args.bevy,args.theia,case/'pose-proof.json')
            pixels=[]
            for label in probe['captures']:
                meta=json.loads((case/label/'screenshot.json').read_bytes());require(meta['selected']==variant['frames'][meta['frame']]['selected'],'Capture/recorded selection differs')
                pixels.append(verify_pixels(observed,case/label,args.bevy))
            summary['cases'].append(dict(name=name,region=region,players=players,phase=expected,report=str(case/'report.json'),poses=poses,pixels=pixels))
        summary['display']=display
    # Fail before GPU setup for invalid plans and protected-coarse short caps.
    (out/'negative/report.json').unlink(missing_ok=True)
    for flags in (['--streaming-probe','4'],['--probe-cpu-budget','1'],['--streaming-probe','2','--probe-cpu-budget',str(encoded_base-1)],['--streaming-probe','2','--probe-upload-budget',str(logical_base-1)]):
        process=subprocess.run([str(args.bevy.resolve()),'--fixture',str(out/'source-fixture.json'),'--output',str(out/'negative'),*flags],capture_output=True,text=True,timeout=30)
        require(process.returncode!=0 and not (out/'negative/report.json').exists(),'Invalid probe produced success')
        summary['negative'].append(dict(flags=flags,returncode=process.returncode,stderr=process.stderr))
    broken=out/'broken-source';broken.mkdir(exist_ok=True)
    cook=scene['quadrants'][2]['cook']
    for record in (cook['bundle'],cook,cook['materials'],cook['uv'],cook['texture_table'],*cook['textures']):
        source=Path(record['path']);shutil.copyfile(source,broken/source.name)
    child=broken/Path(cook['uv']['path']).name;data=bytearray(child.read_bytes());data[-1]^=1;child.write_bytes(data)
    invalid=json.loads(json.dumps(fixture));invalid['source_bundles'][2]=str(broken/Path(cook['bundle']['path']).name)
    invalid_path=out/'broken-fixture.json';invalid_path.write_text(json.dumps(invalid))
    process=subprocess.run([str(args.bevy.resolve()),'--fixture',str(invalid_path),'--output',str(out/'negative'),'--streaming-probe','2'],capture_output=True,text=True,timeout=30)
    require(process.returncode!=0 and ('digest' in process.stderr or 'hash' in process.stderr) and not (out/'negative/report.json').exists(),'Corrupt cooked source was admitted')
    summary['negative'].append(dict(kind='startup source identity failure',returncode=process.returncode,stderr=process.stderr))
    summary.update(passed=True,encoded_base=encoded_base,logical_base=logical_base,source_fixture_sha256=hashlib.sha256((out/'source-fixture.json').read_bytes()).hexdigest(),executable_sha256=hashlib.sha256(args.bevy.read_bytes()).hexdigest())
    result.write_text(json.dumps(summary,indent=2)+'\n');print('Qualified:',result,flush=True)


if __name__=='__main__':main()
