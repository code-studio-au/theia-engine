#!/usr/bin/env python3
"""Record stats-display on/off profiles for Theia's GPU HUD and native Bevy UI."""
import argparse
import html
import json
from pathlib import Path
import subprocess
import sys

from world_camera_reference import require
ROOT=Path(__file__).resolve().parents[1]


def analyse(output):
    controls=[json.loads((output/mode/'comparison.json').read_bytes()) for mode in ('off','on')]
    require(all(c['completed'] for c in controls) and not controls[0]['stats_overlay'] and controls[1]['stats_overlay'],'Missing on/off controls')
    require(controls[0].get('present_mode','fifo')==controls[1].get('present_mode','fifo') and controls[0].get('gpu_timings',False)==controls[1].get('gpu_timings',False),'Presentation/instrumentation controls differ')
    require(controls[0].get('independent_streaming',False)==controls[1].get('independent_streaming',False) and controls[0].get('target_hz')==controls[1].get('target_hz'),'Streaming/clock controls differ')
    require(len(controls[0]['cases'])==len(controls[1]['cases']),'Control population sets differ')
    rows=[]
    for off_case,on_case in zip(controls[0]['cases'],controls[1]['cases']):
        require(off_case['players']==on_case['players'],'Population differs')
        for engine,key,metrics in [('Theia','theia_report',[('gpu_resolve_ms','GPU resolve'),('gpu_total_ms','GPU frame'),('cpu_draw_ms','CPU draw wall'),('frame_interval_ms','Host interval')]),
                ('Bevy','bevy_report',[('cpu_main_schedule_ms','CPU main schedule'),('frame_interval_ms','Host interval')])]:
            reports=[json.loads(Path(c[key]).read_bytes()) for c in (off_case,on_case)]
            require(reports[0]['frames']==reports[1]['frames'] and reports[0]['warmup_frames']==reports[1]['warmup_frames'],'Sampling differs')
            manifest=lambda r:r['launcher']['source_manifest_sha256'] if engine=='Theia' else r['source_manifest_sha256']
            require(manifest(reports[0])==manifest(reports[1]),'Control source assets differ')
            if controls[0].get('independent_streaming'):
                if engine=='Bevy':
                    require(reports[0]['streaming_probe']['camera']['slots']==reports[1]['streaming_probe']['camera']['slots'] and reports[0]['streaming_probe']['cpu_budget']==reports[1]['streaming_probe']['cpu_budget'],'Streaming control slots/caps differ')
                else:
                    require(reports[0]['metadata']['camera_route_frames']==reports[1]['metadata']['camera_route_frames'] and reports[0]['metadata']['bundle_cpu_budget']==reports[1]['metadata']['bundle_cpu_budget'],'Streaming control route/caps differ')
            selected=lambda s:s['world']['selected'] if engine=='Theia' else s['selected']
            warmup=reports[0]['warmup_frames']
            pairs=[(a,b) for a,b in zip(reports[0]['samples'][warmup:],reports[1]['samples'][warmup:]) if a['frame']==b['frame'] and a['scene_time']==b['scene_time'] and selected(a)==selected(b)]
            require(len(pairs)>=3,'Insufficient frames with matching scene selections')
            if engine=='Bevy' and controls[0].get('gpu_timings'):metrics=[('gpu_render_graph_ms','GPU render graph'),*metrics]
            for metric,label in metrics:
                off=sum(a[metric] for a,b in pairs)/len(pairs);on=sum(b[metric] for a,b in pairs)/len(pairs)
                rows.append(dict(players=off_case['players'],engine=engine,metric=label,metric_key=metric,matched_frames=len(pairs),off_ms=off,on_ms=on,observed_change_ms=on-off))
    result=dict(completed=True,independent_streaming=controls[0].get('independent_streaming',False),target_hz=controls[0].get('target_hz'),controls=[str(output/mode/'comparison.json') for mode in ('off','on')],rows=rows,
        scope='Observed profile changes in frames with the same logical clock and selected geometry. Streaming/upload phases can still differ. Presentation, run order, GPU clocks and other renderer work affect observations. Theia replaces scene resolve inside the panel; Bevy draws UI after the scene. Different fonts/statistics and Theia descriptor profiles mean this is not an isolated font microbenchmark or a general UI ranking. Bevy GPU time requires explicit instrumentation; CPU metric scopes differ between engines.')
    (output/'stats-comparison.json').write_text(json.dumps(result,indent=2)+'\n')
    body=''.join('<tr><td>%d</td><td>%s</td><td>%s</td><td>%d</td><td>%.4f</td><td>%.4f</td><td>%+.4f</td></tr>'%(r['players'],r['engine'],r['metric'],r['matched_frames'],r['off_ms'],r['on_ms'],r['observed_change_ms']) for r in rows)
    page='''<!doctype html><html lang="en"><meta charset="utf-8"><title>Stats display comparison</title><style>body{font:16px system-ui;max-width:1100px;margin:3rem auto;padding:1rem;background:#111923;color:#e5edf5}td,th{padding:.5rem;text-align:right}a{color:#9cd}</style><h1>Stats display: off / on</h1><p>Theia: fixed GPU debug font in material resolve. Bevy: ordinary UI text, default font and layout. Both panels occupy 468×288 pixels and update every frame. Their available statistics and glyphs differ.</p><p>SCOPE</p><table><tr><th>Proxies</th><th>Engine</th><th>Metric</th><th>Matched frames</th><th>Off ms</th><th>On ms</th><th>Observed change ms</th></tr>ROWS</table><p>Each row compares one engine with its own control. Do not compare the differently scoped CPU rows as engine rankings.</p><p><a href="off/comparison.html">Overlay off traces</a> · <a href="on/comparison.html">Overlay on traces</a> · <a href="stats-comparison.json">JSON and provenance paths</a></p></html>'''
    (output/'stats-comparison.html').write_text(page.replace('SCOPE',html.escape(result['scope'])).replace('ROWS',body))
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True);parser.add_argument('--output',type=Path,default=ROOT/'build/stats-comparison')
    parser.add_argument('--players',type=int,nargs='+',default=[10000]);parser.add_argument('--frames',type=int,default=660)
    parser.add_argument('--warmup',type=int,default=60);parser.add_argument('--route-frames',type=int,default=600)
    parser.add_argument('--pre-roll',type=int,default=120);parser.add_argument('--gpu');parser.add_argument('--headless',action='store_true');parser.add_argument('--headless-dri-prime')
    parser.add_argument('--present-mode',choices=['fifo','immediate','mailbox'],default='fifo');parser.add_argument('--gpu-timings',action='store_true')
    parser.add_argument('--independent-streaming',action='store_true');parser.add_argument('--target-hz',type=int)
    parser.add_argument('--fine-slots',type=int,choices=range(1,5));parser.add_argument('--stream-cpu-budget',type=int)
    parser.add_argument('--validation',action='store_true');parser.add_argument('--skip-build',action='store_true');parser.add_argument('--slangc')
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    for name in ('stats-comparison.json','stats-comparison.html'):(out/name).unlink(missing_ok=True)
    common=[sys.executable,str(ROOT/'tools/run_bevy_comparison.py'),'--scene',str(args.scene.resolve()),'--players',*[str(p) for p in args.players],'--frames',str(args.frames),'--warmup',str(args.warmup),'--route-frames',str(args.route_frames),'--pre-roll',str(args.pre_roll)]
    for flag in ('headless','validation','gpu_timings','independent_streaming'):
        if getattr(args,flag):common+=['--'+flag.replace('_','-')]
    for name in ('gpu','headless_dri_prime','slangc','present_mode','target_hz','fine_slots','stream_cpu_budget'):
        value=getattr(args,name)
        if value is not None:common+=['--'+name.replace('_','-'),str(value)]
    for enabled in (False,True):
        mode='on' if enabled else 'off';command=[*common,'--output',str(out/mode)]
        if args.skip_build or enabled:command+=['--skip-build']
        if enabled:command+=['--stats-overlay']
        print('Recording stats display',mode,flush=True)
        with (out/(mode+'.log')).open('w') as log:subprocess.run(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=max(900,args.frames/5,4*args.frames/(args.target_hz or 240)+120))
    analyse(out);print('Stats display comparison:',out/'stats-comparison.html',flush=True)


if __name__=='__main__':main()
