#!/usr/bin/env python3
"""Qualify capture-free paired streaming, timing tags, pressure and preserved diagnostic/replay paths."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from headless_display import owned_display
from qualify_bevy_camera_stream import check
from qualify_bevy_workload import verify as verify_poses
from run_bevy_comparison import run
from verify_bevy_snapshot import verify as verify_pixels
from world_camera_reference import Oracle,require

ROOT=Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=ROOT/'build/qualification/bevy-stream-measure/native')
    parser.add_argument('--headless-dri-prime');parser.add_argument('--gpu',default='7900')
    args=parser.parse_args();os.chdir(ROOT)
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';summary=dict(passed=False,cases=[],negative=[]);result.write_text(json.dumps(summary)+'\n')
    bevy=ROOT/'tools/theia-bevy-benchmark/target/release/theia-bevy-benchmark'
    theia=ROOT/'apps/theia-render-lab/target/release/theia-render-lab'
    for name,players,frames,flags in [('paired',['0','10000'],240,[]),('hud',['10000'],180,['--stats-overlay']),
                                      ('one-slot',['10000'],180,['--fine-slots','1']),
                                      ('coarse',['10000'],180,['--stream-cpu-budget','142040'])]:
        command=[sys.executable,'tools/run_bevy_comparison.py','--scene',str(args.scene.resolve()),'--output',str(out/name),
                 '--players',*players,'--frames',str(frames),'--route-frames','120','--warmup','10','--pre-roll','60',
                 '--independent-streaming','--target-hz','60','--present-mode','immediate','--gpu-timings','--validation','--headless','--skip-build','--gpu',args.gpu,*flags]
        if args.headless_dri_prime:command+=['--headless-dri-prime',args.headless_dri_prime]
        run(command,os.environ.copy(),out/(name+'.log'),240)
        paired=json.loads((out/name/'comparison.json').read_bytes())
        require(paired['completed'] and paired['independent_streaming'] and not paired['visual_route_match'],'Wrong pair mode')
        for case in paired['cases']:
            report_path=Path(case['bevy_report']);report=json.loads(report_path.read_bytes())
            require(not report['streaming_probe']['captures'] and not list(report_path.parent.rglob('screenshot.raw')),'Measured run captured images')
            rows=list(csv.DictReader(report_path.with_suffix('.csv').open()))
            require(len(rows)==frames and all(json.loads(row['stream_probe'])==sample['stream_probe'] and json.loads(row['frame_pacing'])==sample['frame_pacing'] for row,sample in zip(rows,report['samples'])),'CSV streaming/clock round trip differs')
            if name=='coarse':
                require(case['streaming_proof']['publications']==0 and case['streaming_proof']['cpu_deferrals']>0,'Coarse CPU cap not exercised')
                for engine in ('theia','bevy'):
                    require(case['streaming_comparison'][engine]['unresolved_windows']>0 and case['streaming_comparison'][engine]['missing_near_detail_region_frames']>0,'Missing/unresolved detail was hidden')
            else:require(case['streaming_proof']['publications']>=4,'Repeated publication not exercised')
            if name=='hud':require(case['hud_proof']['frames']==frames,'HUD was not checked per frame')
        summary['cases'].append(dict(name=name,comparison=str(out/name/'comparison.json')))
    # Unmodified diagnostic captures remain separate from measured runs.
    fixture_path=out/'paired/01-10000-proxies/fixture.json';fixture=json.loads(fixture_path.read_bytes())
    logical=[sum(len(p['positions'])*32+len(p['texture']['rgba'])+32 for p in c['parts']) for c in fixture['chunks']]
    oracle=Oracle(json.loads(args.scene.read_bytes()),theia)
    with owned_display(out/'display',args.headless_dri_prime) as (native,_,display):
        env=native.copy();env.update(WGPU_ADAPTER_NAME=args.gpu,WGPU_VALIDATION='1',WGPU_DEBUG='1',VK_INSTANCE_LAYERS='VK_LAYER_KHRONOS_validation')
        case=out/'diagnostic';case.mkdir(exist_ok=True)
        run([str(bevy),'--fixture',str(fixture_path),'--output',str(case),'--camera-stream','--pre-roll','60','--present-mode','immediate','--target-hz','60','--stream-capture-frames','0,75,239'],env,case/'run.log')
        report=json.loads((case/'report.json').read_bytes());proof=check(report,fixture,oracle,logical)
        for row,sample in zip(fixture['frames'],report['samples']):row['selected']=sample['selected']
        observed=case/'observed-fixture.json';observed.write_text(json.dumps(fixture,separators=(',',':'))+'\n')
        poses=verify_poses(observed,bevy,theia,case/'pose-proof.json')
        pixels=[verify_pixels(observed,case/label,bevy,exclude_ambiguous_proxy_depth=True) for label in report['streaming_probe']['captures']]
        summary['diagnostic']=dict(proof=proof,poses=poses,pixels=pixels);summary['display']=display
    replay=out/'replay'
    command=[sys.executable,'tools/run_bevy_comparison.py','--scene',str(args.scene.resolve()),'--output',str(replay),'--players','10000','--frames','60','--warmup','10','--pre-roll','60','--gpu-timings','--stats-overlay','--snapshot-frames','0','59','--present-mode','immediate','--validation','--headless','--skip-build','--gpu',args.gpu]
    if args.headless_dri_prime:command+=['--headless-dri-prime',args.headless_dri_prime]
    run(command,os.environ.copy(),out/'replay.log',240)
    summary['replay']=str(replay/'comparison.json')
    for flags in (['--stream-measure'],['--camera-stream','--stream-measure','--stream-capture-frames','0'],
                  ['--camera-stream','--stream-measure','--snapshot','0'],['--camera-stream','--stream-measure','--reference','0'],
                  ['--camera-stream','--gpu-timings'],['--streaming-probe','2','--stream-measure']):
        process=subprocess.run([str(bevy),'--fixture',str(fixture_path),'--output',str(out/'negative'),*flags],capture_output=True,text=True,timeout=30)
        require(process.returncode!=0 and not (out/'negative/report.json').exists(),'Invalid stream mode produced success');summary['negative'].append(flags)
    summary.update(passed=True,provenance={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (theia,bevy)})
    result.write_text(json.dumps(summary,indent=2)+'\n');print('Qualified:',result,flush=True)


if __name__=='__main__':main()
