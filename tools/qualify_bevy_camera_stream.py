#!/usr/bin/env python3
"""Check repeated native Bevy demand, GPU publication and retirement independently."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from export_bevy_benchmark import export
from headless_display import owned_display
from qualify_bevy_workload import verify as verify_poses
from verify_bevy_snapshot import verify as verify_pixels
from world_camera_reference import Oracle, require
ROOT=Path(__file__).resolve().parents[1]


def check(report,fixture,oracle,logical):
    final=report['streaming_probe'];camera=final['camera'];slots=camera['slots'];base=set(range(4,8));previous=set();epoch=1
    require(report['workload']=='theia-bevy-camera-stream-v1' and report['streaming'] and not report['gpu_timestamps'] and not report['overlay'],'Wrong correctness profile')
    require(len(report['samples'])==len(fixture['frames']),'Incomplete recording')
    def ledger(state):
        committed=set(state['committed_fine']);pending={state['region']} if state['pending_encoded'] else set();ids=base|committed|pending
        require(len(committed)<=slots and state['protected_epochs']==[1]*4,'Fine slots/protected epoch differs')
        require(state['cpu_unique_bytes']==oracle.charge(ids)<=state['cpu_budget'],'Encoded ownership/cap differs')
        require(state['logical_asset_bytes']==sum(logical[i] for i in ids)<=state['logical_budget'],'Adapted logical ownership/cap differs')
        require(state['physical_gpu_bytes'] is None,'Logical bytes mislabelled as VRAM')
        selected=[r if r in committed else r+4 for r in range(4)]
        if state['phase'] in ('retiring','unloading') and not state['pending_encoded']:selected[state['region']]=state['region']+4
        require(state['selected']==selected,'Missing coverage or premature/stale fine selection')
        return committed
    counters={k:0 for k in ('requests','cpu_deferrals','logical_deferrals')};blocked={}
    for index,(sample,row) in enumerate(zip(report['samples'],fixture['frames'])):
        state=sample['stream_probe'];plan=state['camera']['plan'];current=ledger(state)
        expected=oracle.plan(row['eye'],previous,slots)
        require(plan['eye']==row['eye'] and {k:plan[k] for k in expected}==expected,'Independent distance/priority plan differs')
        require(sample['frame']==index and sample['scene_time']==row['time'] and sample['selected']==state['selected'],'Frame/view differs')
        for key in counters:
            delta=state['camera'][key]-counters[key];require(delta in (0,1),'Unbounded request/deferral counter')
            if delta:
                candidate=state['region']
                if key=='requests':
                    eligible=[r for r in expected['loads'] if blocked.get(r)!=epoch]
                    require(not expected['removals'] and eligible and candidate==eligible[0] and state['phase']=='loading','Blocked/priority request differs')
                else:
                    ids=base|previous|{candidate}
                    require(candidate in expected['loads'],'Obsolete candidate deferred instead of cancelled')
                    if key=='cpu_deferrals':require(oracle.charge(ids)>state['cpu_budget'],'False CPU deferral')
                    else:require(oracle.charge(ids)<=state['cpu_budget'] and sum(logical[r] for r in ids)>state['logical_budget'],'False logical deferral')
                    blocked[candidate]=epoch
            counters[key]=state['camera'][key]
        added=current-previous;removed=previous-current
        require(len(added)+len(removed)<=1,'More than one publication/retirement in a frame')
        if added:
            require(state['phase']=='idle' and state['gpu_ready'] and added<=set(expected['loads']),'GPU-unready or unwanted publication')
        epoch+=len(added)+len(removed)
        require(state['cpu_epoch']==epoch==1+state['camera']['publications']+state['camera']['evictions'],'Publication epoch/counters differ')
        previous=current
    require(not ledger(final) and final['selected']==[4,5,6,7] and final['phase']=='idle' and camera['ending'] and not final['pending_encoded'],'Final drain left owners/candidate')
    require(final['cpu_unique_bytes']==final['base_cpu_bytes'] and final['logical_asset_bytes']==final['base_logical_asset_bytes'],'Final base charge differs')
    require(final['cpu_epoch']==1+camera['publications']+camera['evictions'] and camera['publications']==camera['evictions'],'Final CPU epoch differs')
    history=camera['history'];require(0<len(history)<=64,'Unbounded/absent history')
    gpu_cancels=camera['cancellations']-camera['worker_cancellations']
    expected_events=1+camera['requests']+camera['uploads']+camera['publications']+3*camera['evictions']+3*gpu_cancels+camera['worker_cancellations']+camera['cpu_deferrals']+camera['logical_deferrals']
    require(len(history)+camera['dropped_history']==expected_events,'History ring/event count differs')
    last=-1
    for row in history:
        ledger(row);require(last<=row['frame']<len(fixture['frames'])+600,'History frame order differs');last=row['frame']
    return dict(frames=len(report['samples']),events_checked=len(history),dropped_history=camera['dropped_history'],publications=camera['publications'],evictions=camera['evictions'],cancellations=camera['cancellations'],ready_cancellations=camera['ready_cancellations'],cpu_deferrals=camera['cpu_deferrals'],logical_deferrals=camera['logical_deferrals'],peak_encoded=max(s['stream_probe']['cpu_unique_bytes'] for s in report['samples']),peak_logical=max(s['stream_probe']['logical_asset_bytes'] for s in report['samples']))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True);parser.add_argument('--output',type=Path,default=ROOT/'build/qualification/bevy-camera-stream')
    parser.add_argument('--headless-dri-prime');parser.add_argument('--gpu',default='7900')
    args=parser.parse_args();os.chdir(ROOT);out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';summary=dict(passed=False,cases=[],negative=[],scope='Camera-demand correctness; no physical-memory or streaming-throughput equivalence');result.write_text(json.dumps(summary)+'\n')
    bevy=ROOT/'tools/theia-bevy-benchmark/target/release/theia-bevy-benchmark';theia=ROOT/'apps/theia-render-lab/target/release/theia-render-lab'
    scene=json.loads(args.scene.read_bytes());oracle=Oracle(scene,theia)
    def run(command,env,log):
        with log.open('w') as file:subprocess.run(command,env=env,stdout=file,stderr=subprocess.STDOUT,check=True,timeout=240)
        require(not any(x in log.read_text() for x in ('VUID-','Validation Error',' ERROR ','panicked at','Falling back to')),'Runtime diagnostic: '+str(log))
    with owned_display(out/'display',args.headless_dri_prime) as (native,_,display):
        env=native.copy();env.update(WGPU_ADAPTER_NAME=args.gpu,WGPU_VALIDATION='1',WGPU_DEBUG='1',VK_INSTANCE_LAYERS='VK_LAYER_KHRONOS_validation')
        fixtures={}
        for name,frames,period in [('route',1320,120),('rapid',180,12)]:
            directory=out/(name+'-source')
            command=[sys.executable,'tools/run_scene_benchmark.py','--streamed-world','--cooked-scene',str(args.scene.resolve()),'--players','10000','--frames',str(frames),'--route-frames',str(period),'--warmup','10','--no-overlay','--present-mode','immediate','--validation','--skip-build','--gpu',args.gpu,'--output',str(directory)]
            run(command,native,out/(name+'-source.log'))
            source=directory/'00-streamed-10000-proxies/report.json'
            fixtures[name]=export(args.scene,source,out/(name+'-fixture.json'),theia)
        logical=[sum(len(p['positions'])*32+len(p['texture']['rgba'])+32 for p in c['parts']) for c in fixtures['route']['chunks']]
        base=set(range(4,8));base_cpu=oracle.charge(base);base_logical=sum(logical[4:]);one_cpu=max(oracle.charge(base|{r}) for r in range(4));one_logical=base_logical+max(logical[:4])
        plans=[('zero','route',0,660,[],False),('crowd','route',10000,660,[],False),('one-slot','route',10000,660,['--stream-fine-slots','1'],False),
            ('cpu-tight','route',10000,660,['--stream-cpu-budget',str(one_cpu)],False),('logical-tight','route',10000,660,['--stream-upload-budget',str(one_logical)],False),
            ('cpu-coarse','route',10000,660,['--stream-cpu-budget',str(base_cpu)],False),('logical-coarse','route',10000,660,['--stream-upload-budget',str(base_logical)],False),
            ('rapid','rapid',10000,180,[],False),('long','route',10000,1320,[],True)]
        for name,source,population,frames,flags,long in plans:
            print('Qualifying',name,flush=True);case=out/name;case.mkdir(exist_ok=True);(case/'report.json').unlink(missing_ok=True)
            fixture=json.loads(json.dumps(fixtures[source]));fixture['frames']=fixture['frames'][:frames];fixture['players']=population;fixture['scope']='Camera-stream input; output selections are observed Bevy state, not Theia replay'
            path=case/'fixture.json';path.write_text(json.dumps(fixture,separators=(',',':'))+'\n')
            # Capture six distinct points across the first two traversals and the
            # endpoint; the long pressure case omits captures to keep it bounded.
            captures=[] if long else sorted({0,min(30,frames-1),min(75,frames-1),min(119,frames-1),min(155,frames-1),frames-1})
            capture_flags=['--stream-capture-frames',','.join(map(str,captures))] if captures else []
            if long:capture_flags=['--stream-capture-frames','0']
            run([str(bevy),'--fixture',str(path),'--output',str(case),'--pre-roll','60','--present-mode','immediate','--camera-stream',*capture_flags,*flags],env,case/'run.log')
            report=json.loads((case/'report.json').read_bytes());proof=check(report,fixture,oracle,logical)
            if name in ('zero','crowd','one-slot','long'):require(proof['publications']>=8,'Repeated publication not exercised')
            if name=='long':require(proof['dropped_history']>0,'History wrap not exercised')
            if name=='cpu-coarse':require(not proof['publications'] and proof['cpu_deferrals']>0,'CPU fallback pressure not exercised')
            if name=='logical-coarse':require(not proof['publications'] and proof['logical_deferrals']>0,'Logical fallback pressure not exercised')
            if name=='rapid':require(proof['cancellations']>0,'Rapid cancellation not exercised')
            for row,sample in zip(fixture['frames'],report['samples']):row['selected']=sample['selected']
            observed=case/'observed-fixture.json';observed.write_text(json.dumps(fixture,separators=(',',':'))+'\n')
            poses=verify_poses(observed,bevy,theia,case/'pose-proof.json');pixels=[]
            for label in report['streaming_probe']['captures']:
                meta=json.loads((case/label/'screenshot.json').read_bytes());require(meta['selected']==fixture['frames'][meta['frame']]['selected'],'Capture selection differs')
                pixels.append(verify_pixels(observed,case/label,bevy,exclude_ambiguous_proxy_depth=True))
            summary['cases'].append(dict(name=name,report=str(case/'report.json'),proof=proof,poses=poses,pixels=pixels))
        summary['display']=display
    for flags in (['--stream-fine-slots','0'],['--camera-stream','--stream-fine-slots','5'],['--camera-stream','--streaming-probe','2'],['--camera-stream','--stream-capture-frames','0,0'],['--camera-stream','--gpu-timings']):
        process=subprocess.run([str(bevy),'--fixture',str(out/'route-fixture.json'),'--output',str(out/'negative'),*flags],capture_output=True,text=True,timeout=30)
        require(process.returncode!=0 and not (out/'negative/report.json').exists(),'Invalid plan produced success');summary['negative'].append(dict(flags=flags,returncode=process.returncode))
    summary.update(passed=True,executable_sha256=hashlib.sha256(bevy.read_bytes()).hexdigest());result.write_text(json.dumps(summary,indent=2)+'\n');print('Qualified:',result,flush=True)


if __name__=='__main__':main()
