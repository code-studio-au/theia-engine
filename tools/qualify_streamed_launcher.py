#!/usr/bin/env python3
"""Qualify public launcher routes, export fidelity and failed-rerun cleanup on an owned display."""
import argparse
import csv
import json
import os
from pathlib import Path
import subprocess
import sys

from world_camera_reference import require
from world_timing_reference import verify

ROOT=Path(__file__).resolve().parents[1]


def check_exports(path):
    report=json.loads(path.read_bytes())
    with path.with_suffix('.csv').open(newline='') as file:rows=list(csv.DictReader(file))
    require(len(rows)==len(report['samples']),'CSV sample count differs')
    for sample,row in zip(report['samples'],rows):
        for key in ('frame','scene_time','gpu_total_ms','cpu_draw_ms'):
            require(float(row[key])==sample[key],'CSV numeric sample differs: '+key)
        if 'world' in sample:
            world=sample['world'];pending=world['pending'] or {}
            expected=dict(world_epoch=world['epoch'],world_selected=json.dumps(world['selected'],separators=(',',':')),
                world_publications=world['publications'],world_evictions=world['evictions'],world_deferrals=world['deferrals'],
                world_cpu_unique_bytes=world['cpu_unique_bytes'],world_graph_gpu_bytes=world['physical_bytes'],
                world_pending_gpu_bytes=pending.get('pending_gpu_bytes',0),world_fixed_gpu_bytes=world['fixed_gpu_bytes'],
                world_total_gpu_bytes=world['total_gpu_bytes'],world_image_bytes=world['image_bytes'],
                world_pending_image_bytes=pending.get('pending_image_bytes',0),world_selected_images=world['selected_images'],
                world_owned_chunks=world['owned_chunks'],world_route_cycle=world['route_cycle'])
            require(all(row[key]==str(value) for key,value in expected.items()),'CSV world ledger differs')
    page=path.with_suffix('.html').read_text()
    script=page.split('<script>',1)[1].split('</script>',1)[0]
    require(json.loads(script.split('const samples=',1)[1].split(',c=document',1)[0])==[s for s in report['samples'] if not s['warmup']],'HTML measured samples differ')
    if 'world' in report['samples'][0]:require('world.total_gpu_bytes' in page and 'world.cpu_unique_bytes' in page and 'selected '+"'" in page,'Missing world graph/tooltip')
    # Optional syntax check uses a system tool, never installs a JS runtime.
    import shutil
    if shutil.which('node'):
        subprocess.run(['node','--check','--input-type=commonjs'],input=script,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--resident-scene',type=Path,required=True)
    parser.add_argument('--gpu');parser.add_argument('--headless-dri-prime')
    args=parser.parse_args();os.chdir(ROOT)
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True);(out/'report.json').unlink(missing_ok=True)
    scene=json.loads(args.scene.read_bytes());renderer=ROOT/'apps/theia-render-lab/target/release/theia-render-lab'
    common=[sys.executable,str(ROOT/'tools/run_scene_benchmark.py'),'--skip-build','--headless','--validation','--frames','180','--warmup','10']
    if args.gpu:common+=['--gpu',args.gpu]
    if args.headless_dri_prime:common+=['--headless-dri-prime',args.headless_dri_prime]
    cases=[('streamed',['--streamed-world','--cooked-scene',str(args.scene.resolve()),'--players','0','10000']),
        ('tight-repeat',['--streamed-world','--cooked-scene',str(args.scene.resolve()),'--no-overlay','--route-frames','60','--fine-slots','1','--bundle-gpu-budget','464592','--bundle-byte-budget','166164','--texture-budget','327680']),
        ('procedural',['--players','100']),
        ('resident',['--cooked-scene',str(args.resident_scene.resolve()),'--whole-world']),
        ('mixed',['--cooked-scene',str(args.resident_scene.resolve()),'--whole-world','--mixed-world','--players','10000'])]
    results=[]
    for name,flags in cases:
        output=out/name
        with (out/(name+'.log')).open('w') as log:
            subprocess.run([*common,*flags,'--output',str(output)],stdout=log,stderr=subprocess.STDOUT,check=True,timeout=180)
        suite=json.loads((output/'suite.json').read_bytes())
        for entry in suite:
            path=Path(entry['report']);report=check_exports(path);checks={}
            if report['workload']=='theia-streamed-mmo-v1':
                metadata=report['metadata'];caps=[metadata[k] for k in ('bundle_cpu_budget','aggregate_gpu_budget','image_gpu_budget')]
                checks=verify(report,json.loads((path.parent/'camera.json').read_bytes()),scene,renderer,180,
                    metadata['camera_route_frames'],10,report['players'],caps,report['overlay'])
            results.append(dict(case=name,report=str(path),export_fidelity=True,**checks))
        print('Passed launcher route/export:',name,flush=True)
    # Rerun a successful owned output with an impossible coarse bootstrap cap.
    import shutil
    output=out/'failed-rerun'
    if output.exists():shutil.rmtree(output)
    shutil.copytree(out/'streamed',output)
    with (out/'failed-rerun.log').open('w') as log:
        failed=subprocess.run([*common,*cases[0][1],'--bundle-gpu-budget','375071','--output',str(output)],stdout=log,stderr=subprocess.STDOUT,timeout=120)
    first=output/'00-streamed-0-proxies'
    require(failed.returncode!=0 and not (output/'suite.json').exists() and all(not (first/('report'+suffix)).exists() for suffix in ('.json','.csv','.html')),'Failed rerun retained a completed owned result')
    rejected=[['--streamed-world','--whole-world'],['--streamed-world','--seconds','30'],['--route-frames','60'],['--fine-slots','1'],['--streamed-world','--players','10001']]
    for flags in rejected:
        result=subprocess.run([sys.executable,str(ROOT/'tools/run_scene_benchmark.py'),*flags],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,timeout=10)
        require(result.returncode==2,'Invalid launcher arguments were not rejected before execution')
    (out/'report.json').write_text(json.dumps(dict(passed=True,routes=results,failed_rerun_cleanup=True,preflight_rejections=len(rejected)),indent=2)+'\n')


if __name__=='__main__':main()
