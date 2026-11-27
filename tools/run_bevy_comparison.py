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
    if any(marker in text for marker in ('VUID-','Validation Error',' ERROR ','panicked at','Falling back to')):raise RuntimeError('Runtime diagnostic in '+str(log))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=ROOT/'build/bevy-comparison')
    parser.add_argument('--players',type=int,nargs='+',default=[1000,10000])
    parser.add_argument('--frames',type=int,default=3660);parser.add_argument('--warmup',type=int,default=60)
    parser.add_argument('--route-frames',type=int);parser.add_argument('--pre-roll',type=int,default=120)
    parser.add_argument('--width',type=int,default=1280);parser.add_argument('--height',type=int,default=720)
    parser.add_argument('--gpu');parser.add_argument('--headless',action='store_true');parser.add_argument('--headless-dri-prime')
    parser.add_argument('--gpu-timings',action='store_true',help='collect Bevy render-graph GPU intervals with exact logical frame tags and serial completion')
    parser.add_argument('--present-mode',choices=['fifo','immediate','mailbox'],default='fifo',help='exact mode for both renderers; unsupported modes fail')
    parser.add_argument('--target-hz',type=int,help='optional shared 1..240 Hz schedule; inspect recorded lateness before interpreting I/O deadlines')
    parser.add_argument('--independent-streaming',action='store_true',help='capture-free independent cooked streaming; requires --target-hz and at least 140 frames')
    parser.add_argument('--fine-slots',type=int,choices=range(1,5),help='shared streaming residency slots (default 2); independent streaming only')
    parser.add_argument('--stream-cpu-budget',type=int,help='shared unique encoded budget; GPU/logical budgets remain backend-specific')
    parser.add_argument('--stats-overlay',action='store_true',help='show Theia GPU HUD and a corresponding native Bevy UI stats panel')
    parser.add_argument('--validation',action='store_true')
    parser.add_argument('--skip-build',action='store_true',help='reuse both release executables and existing Theia SPIR-V')
    parser.add_argument('--slangc',default=shutil.which('slangc'))
    parser.add_argument('--snapshot-frames',type=int,nargs='*',default=[],help='additional frozen Bevy frames with independent source-pixel checks; excluded from timing')
    args=parser.parse_args();os.chdir(ROOT)
    if args.target_hz is not None and not 1<=args.target_hz<=240:parser.error('Target Hz must be 1..240')
    if args.independent_streaming and (args.target_hz is None or args.frames<140 or args.snapshot_frames):parser.error('Independent streaming requires --target-hz, at least 140 frames and no frozen snapshots')
    if not args.independent_streaming and (args.fine_slots is not None or args.stream_cpu_budget is not None):parser.error('Streaming budgets/slots require --independent-streaming')
    if args.stream_cpu_budget is not None and args.stream_cpu_budget<1:parser.error('Streaming CPU budget must be positive')
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
            command+=['--present-mode',args.present_mode]
            if args.target_hz is not None:command+=['--target-hz',str(args.target_hz)]
            if args.independent_streaming:command+=['--fine-slots',str(args.fine_slots or 2)]
            if args.stream_cpu_budget is not None:command+=['--bundle-byte-budget',str(args.stream_cpu_budget)]
            if not args.stats_overlay:command+=['--no-overlay']
            if args.route_frames:command+=['--route-frames',str(args.route_frames)]
            if args.gpu:command+=['--gpu',args.gpu]
            if args.validation:command+=['--validation']
            print('Running Theia source route:',players,'proxies',flush=True)
            timeout=max(180,args.frames/10,2*args.frames/(args.target_hz or 240)+60)
            run(command,native,case/'theia-launcher.log',timeout)
            source=case/'theia'/('00-streamed-%d-proxies'%players)/'report.json'
            fixture=case/'fixture.json';export(args.scene,source,fixture,theia,args.stats_overlay)
            print('Running Bevy independent stream:' if args.independent_streaming else 'Running Bevy native visual replay:',players,'proxies',flush=True)
            overlay_flags=['--present-mode',args.present_mode]+(['--stats-overlay'] if args.stats_overlay else [])
            pacing_flags=['--target-hz',str(args.target_hz)] if args.target_hz is not None else []
            stream_flags=['--camera-stream','--stream-measure','--stream-fine-slots',str(args.fine_slots or 2)] if args.independent_streaming else []
            if args.stream_cpu_budget is not None:stream_flags+=['--stream-cpu-budget',str(args.stream_cpu_budget)]
            run([str(bevy),'--fixture',str(fixture),'--output',str(case/'bevy'),'--pre-roll',str(args.pre_roll),*overlay_flags,*pacing_flags,*stream_flags,*(['--gpu-timings'] if args.gpu_timings else [])],bevy_env,case/'bevy.log',timeout)
            report=json.loads((case/'bevy/report.json').read_bytes());original=json.loads(source.read_bytes())
            require(report['gpu_timestamps']==args.gpu_timings,'GPU timing instrumentation differs')
            if args.gpu_timings:
                import math
                previous=0
                for sample in report['samples']:
                    start,end=sample['gpu_timestamp_ticks'];period=sample['gpu_timestamp_period_ns'];ms=sample['gpu_render_graph_ms']
                    require(start>=previous and end>start and math.isfinite(period) and period>0 and math.isfinite(ms) and abs((end-start)*period/1e6-ms)<1e-9,'Invalid or misordered GPU query evidence')
                    previous=end
            require(report['metadata']['present_mode']==original['metadata']['present_mode']==args.present_mode,'Presentation modes differ')
            require(report['overlay']==original['overlay']==args.stats_overlay,'Overlay configuration differs')
            require(len(report['samples'])==args.frames,'Incomplete Bevy report')
            require(report['warmup_frames']==original['warmup_frames'],'Warmup differs')
            clock_proof=None
            if args.target_hz is not None:
                from benchmark_clock import verify
                clock_proof={'theia':verify(original,args.target_hz),'bevy':verify(report,args.target_hz)}
            require(report['source_report_sha256']==hashlib.sha256(source.read_bytes()).hexdigest(),'Replay source changed')
            require(report['metadata']['backend']=='Vulkan' and report['metadata']['device_id']==original['metadata']['device_id'] and report['extent']==original['samples'][0]['extent'],'Different GPU/backend/extent')
            streaming_comparison=None;streaming_proof=None;hud_proof=None
            pose_fixture=fixture
            if args.independent_streaming:
                from world_camera_reference import Oracle
                from qualify_bevy_camera_stream import check
                from streaming_comparison import compare,verify_hud
                oracle=Oracle(json.loads(args.scene.read_bytes()),theia)
                exported=json.loads(fixture.read_bytes())
                logical=[sum(len(p['positions'])*32+len(p['texture']['rgba'])+32 for p in c['parts']) for c in exported['chunks']]
                require(report['streaming_probe']['camera']['slots']==(args.fine_slots or 2) and report['streaming_probe']['cpu_budget']==original['metadata']['bundle_cpu_budget'],'Streaming slots/encoded caps differ')
                streaming_proof=check(report,exported,oracle,logical,measurement=True)
                streaming_comparison=compare(exported,original,report,oracle,args.fine_slots or 2)
                if args.stats_overlay:hud_proof=verify_hud(report)
                # Preserve the source fixture. CPU proxy checks must follow Bevy's
                # observed terrain selections rather than the earlier Theia run.
                for row,sample in zip(exported['frames'],report['samples']):row['selected']=sample['selected']
                exported.update(streaming=True,workload='theia-bevy-camera-observed-v1',scope='Observed Bevy independent streaming selections for source-ground/pose verification; not Theia selection replay')
                pose_fixture=case/'observed-fixture.json';pose_fixture.write_text(json.dumps(exported,separators=(',',':'))+'\n')
                with (case/'readiness.csv').open('w',newline='') as file:
                    trace=streaming_comparison['trace'];writer=csv.DictWriter(file,fieldnames=list(trace[0]));writer.writeheader();writer.writerows(trace)
            else:
                for sample,other in zip(report['samples'],original['samples']):
                    require(sample['frame']==other['frame'] and sample['scene_time']==other['scene_time'] and sample['selected']==other['world']['selected'],'Replay clock/selection differs')
            from qualify_bevy_workload import verify as verify_poses
            poses=verify_poses(pose_fixture,bevy,theia,case/'pose-proof.json')
            with (case/'bevy/report.csv').open('w',newline='') as file:
                writer=csv.DictWriter(file,fieldnames=['frame','scene_time','frame_interval_ms','cpu_main_schedule_ms','selected']+(['stream_probe'] if args.independent_streaming else [])+(['hud_text'] if args.independent_streaming and args.stats_overlay else [])+(['frame_pacing'] if args.target_hz is not None else [])+(['gpu_render_graph_ms','gpu_timestamp_ticks','gpu_timestamp_period_ns'] if args.gpu_timings else []));writer.writeheader()
                for sample in report['samples']:
                    row=dict(sample)
                    if 'frame_pacing' in row:row['frame_pacing']=json.dumps(row['frame_pacing'],separators=(',',':'))
                    if 'stream_probe' in row:row['stream_probe']=json.dumps(row['stream_probe'],separators=(',',':'))
                    writer.writerow(row)
            proofs=[]
            for frame in args.snapshot_frames:
                capture=case/('snapshot-%d'%frame);capture.mkdir(exist_ok=True)
                run([str(bevy),'--fixture',str(fixture),'--output',str(capture),'--pre-roll',str(args.pre_roll),'--snapshot',str(frame),*overlay_flags],bevy_env,capture/'run.log')
                from verify_bevy_snapshot import verify
                proofs.append(verify(fixture,capture,bevy))
            records.append(dict(players=players,theia_report=str(source),bevy_report=str(case/'bevy/report.json'),fixture_sha256=hashlib.sha256(fixture.read_bytes()).hexdigest(),pixel_proofs=proofs,pose_proof=poses,clock_proof=clock_proof,streaming_comparison=streaming_comparison,streaming_proof=streaming_proof,hud_proof=hud_proof))
    provenance={str(path.relative_to(ROOT)):hashlib.sha256(path.read_bytes()).hexdigest() for path in [bevy,theia,ROOT/'tools/theia-bevy-benchmark/Cargo.lock',ROOT/'tools/theia-bevy-benchmark/assets/shaders/theia_unlit.wgsl']}
    result=dict(completed=True,visual_route_match=not args.independent_streaming,independent_streaming=args.independent_streaming,stats_overlay=args.stats_overlay,present_mode=args.present_mode,gpu_timings=args.gpu_timings,target_hz=args.target_hz,display=display,validation_requested=args.validation,cases=records,provenance=provenance,
        frame_schedule_within_one_period=(all(proof['frames_at_least_one_period_late']==0 for c in records for proof in c['clock_proof'].values()) if args.target_hz else None),
        cache_policy=('Both prevalidate all eight cooked inputs at startup; subsequent fine workers reopen actual files; OS cache uncontrolled, no cold-cache claim; sequential Theia then export then Bevy runs' if args.independent_streaming else 'Theia prevalidates/streams cooked inputs; Bevy preloads decoded fixture graphs and replays selections'),
        scope=('Independent cooked streaming with shared camera/proxy rules, 20/35 m policy, fine slots and encoded budget. Selections/readiness may differ; GPU payload and timing scopes differ. Per-frame JSON retains ownership and missing near-detail observations. No physical-memory or whole-engine ranking.' if args.independent_streaming else 'Matched geometry/textures/camera/logical times/proxy rules/selected regions. Bevy preloads eight graphs and computes proxy presentation on CPU; Theia streams and computes it on GPU. Host-frame intervals may remain compositor-limited even with non-vsynced presentation; CPU scopes differ. No streaming, memory or total-engine performance ranking.'))
    (out/'comparison.json').write_text(json.dumps(result,indent=2)+'\n')
    from bevy_comparison_report import write
    write(out,result)
    print('Comparison recorded:',out/'comparison.html',flush=True)


if __name__=='__main__':main()
