#!/usr/bin/env python3
"""Run the current Theia streamed benchmark and its isolated native Bevy visual replay."""
import argparse
from contextlib import nullcontext
import csv
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys

from export_bevy_benchmark import export
from headless_display import owned_display
from world_camera_reference import require

ROOT=Path(__file__).resolve().parents[1]


def run(command,env,log,timeout=180):
    with log.open('w') as file:subprocess.run(command,cwd=ROOT,env=env,stdout=file,stderr=subprocess.STDOUT,check=True,timeout=timeout)
    text=log.read_text()
    if any(marker in text for marker in ('VUID-','Validation Error',' ERROR ','panicked at')):raise RuntimeError('Runtime diagnostic in '+str(log))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=ROOT/'build/bevy-comparison')
    parser.add_argument('--players',type=int,nargs='+',default=[1000,10000])
    parser.add_argument('--frames',type=int,default=3660);parser.add_argument('--warmup',type=int,default=60)
    parser.add_argument('--route-frames',type=int);parser.add_argument('--pre-roll',type=int,default=120)
    parser.add_argument('--width',type=int,default=1280);parser.add_argument('--height',type=int,default=720)
    parser.add_argument('--gpu');parser.add_argument('--headless',action='store_true');parser.add_argument('--headless-dri-prime')
    parser.add_argument('--stats-overlay',action='store_true',help='show Theia GPU HUD and a corresponding native Bevy UI stats panel')
    parser.add_argument('--validation',action='store_true')
    parser.add_argument('--skip-build',action='store_true',help='reuse both release executables and existing Theia SPIR-V')
    parser.add_argument('--slangc',default=shutil.which('slangc'))
    parser.add_argument('--snapshot-frames',type=int,nargs='*',default=[],help='additional frozen Bevy frames with independent source-pixel checks; excluded from timing')
    args=parser.parse_args();os.chdir(ROOT)
    if not args.players or any(not 0<=p<=10000 for p in args.players) or not 3<=args.frames<=120000 or not 0<=args.warmup<args.frames-2 or not 2<=args.pre_roll<=1200:parser.error('Invalid bounded population/frame/warmup/pre-roll')
    if any(not 0<=f<args.frames for f in args.snapshot_frames):parser.error('Snapshot outside route')
    if args.stats_overlay and (args.width<480 or args.height<300):parser.error('Full stats display comparison requires at least 480x300')
    if args.headless_dri_prime and not args.headless:parser.error('Headless device selection requires --headless')
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    for name in ('comparison.json','comparison.html'):(out/name).unlink(missing_ok=True)
    env=os.environ.copy();env['WGPU_VALIDATION']='1' if args.validation else '0';env['WGPU_DEBUG']=env['WGPU_VALIDATION']
    if args.gpu:env['WGPU_ADAPTER_NAME']=args.gpu
    if not args.skip_build:
        if not args.slangc:parser.error('Put slangc on PATH or supply --slangc')
        run(['cargo','build','--release','--locked','--manifest-path','tools/theia-bevy-benchmark/Cargo.toml'],env,out/'bevy-build.log',900)
        run([sys.executable,'tools/compile_render_shaders.py','--slangc',args.slangc],env,out/'shaders.log')
        run(['cargo','build','--release','--locked','--manifest-path','apps/theia-render-lab/Cargo.toml'],env,out/'theia-build.log')
    bevy=ROOT/'tools/theia-bevy-benchmark/target/release'/('theia-bevy-benchmark.exe' if os.name=='nt' else 'theia-bevy-benchmark')
    theia=ROOT/'apps/theia-render-lab/target/release'/('theia-render-lab.exe' if os.name=='nt' else 'theia-render-lab')
    records=[]
    context=owned_display(out/'display',args.headless_dri_prime) if args.headless else nullcontext((env,env,None))
    with context as (native,_,display):
        native.update({k:v for k,v in env.items() if k.startswith('WGPU_')})
        bevy_env=native.copy()
        if args.validation:bevy_env['VK_INSTANCE_LAYERS']='VK_LAYER_KHRONOS_validation'
        for number,players in enumerate(args.players):
            case=out/('%02d-%d-proxies'%(number,players));case.mkdir(exist_ok=True)
            for path in (case/'bevy/report.json',case/'bevy/report.csv',case/'fixture.json'):path.unlink(missing_ok=True)
            command=[sys.executable,'tools/run_scene_benchmark.py','--streamed-world','--cooked-scene',str(args.scene.resolve()),'--players',str(players),'--frames',str(args.frames),'--warmup',str(args.warmup),'--width',str(args.width),'--height',str(args.height),'--skip-build','--output',str(case/'theia')]
            if not args.stats_overlay:command+=['--no-overlay']
            if args.route_frames:command+=['--route-frames',str(args.route_frames)]
            if args.gpu:command+=['--gpu',args.gpu]
            if args.validation:command+=['--validation']
            print('Running Theia source route:',players,'proxies',flush=True)
            run(command,native,case/'theia-launcher.log',max(180,args.frames/10))
            source=case/'theia'/('00-streamed-%d-proxies'%players)/'report.json'
            fixture=case/'fixture.json';export(args.scene,source,fixture,theia,args.stats_overlay)
            print('Running Bevy native visual replay:',players,'proxies',flush=True)
            overlay_flags=['--stats-overlay'] if args.stats_overlay else []
            run([str(bevy),'--fixture',str(fixture),'--output',str(case/'bevy'),'--pre-roll',str(args.pre_roll),*overlay_flags],bevy_env,case/'bevy.log',max(180,args.frames/10))
            report=json.loads((case/'bevy/report.json').read_bytes());original=json.loads(source.read_bytes())
            require(report['overlay']==original['overlay']==args.stats_overlay,'Overlay configuration differs')
            require(len(report['samples'])==args.frames,'Incomplete Bevy report')
            require(report['source_report_sha256']==hashlib.sha256(source.read_bytes()).hexdigest(),'Replay source changed')
            require(report['metadata']['backend']=='Vulkan' and report['metadata']['device_id']==original['metadata']['device_id'] and report['extent']==original['samples'][0]['extent'],'Different GPU/backend/extent')
            for sample,other in zip(report['samples'],original['samples']):
                require(sample['frame']==other['frame'] and sample['scene_time']==other['scene_time'] and sample['selected']==other['world']['selected'],'Replay clock/selection differs')
            from qualify_bevy_workload import verify as verify_poses
            poses=verify_poses(fixture,bevy,theia,case/'pose-proof.json')
            with (case/'bevy/report.csv').open('w',newline='') as file:
                writer=csv.DictWriter(file,fieldnames=['frame','scene_time','frame_interval_ms','cpu_main_schedule_ms','selected']);writer.writeheader();writer.writerows(report['samples'])
            proofs=[]
            for frame in args.snapshot_frames:
                capture=case/('snapshot-%d'%frame);capture.mkdir(exist_ok=True)
                run([str(bevy),'--fixture',str(fixture),'--output',str(capture),'--pre-roll',str(args.pre_roll),'--snapshot',str(frame),*overlay_flags],bevy_env,capture/'run.log')
                from verify_bevy_snapshot import verify
                proofs.append(verify(fixture,capture,bevy))
            records.append(dict(players=players,theia_report=str(source),bevy_report=str(case/'bevy/report.json'),fixture_sha256=hashlib.sha256(fixture.read_bytes()).hexdigest(),pixel_proofs=proofs,pose_proof=poses))
    provenance={str(path.relative_to(ROOT)):hashlib.sha256(path.read_bytes()).hexdigest() for path in [bevy,theia,ROOT/'tools/theia-bevy-benchmark/Cargo.lock',ROOT/'tools/theia-bevy-benchmark/assets/shaders/theia_unlit.wgsl']}
    result=dict(completed=True,visual_route_match=True,stats_overlay=args.stats_overlay,display=display,validation_requested=args.validation,cases=records,provenance=provenance,
        scope='Matched geometry/textures/camera/logical times/proxy rules/selected regions. Bevy preloads eight graphs and computes proxy presentation on CPU; Theia streams and computes it on GPU. FIFO host-frame intervals are not uncapped throughput; CPU scopes differ. No streaming, memory or total-engine performance ranking.')
    (out/'comparison.json').write_text(json.dumps(result,indent=2)+'\n')
    from bevy_comparison_report import write
    write(out,result)
    print('Comparison recorded:',out/'comparison.html',flush=True)


if __name__=='__main__':main()
