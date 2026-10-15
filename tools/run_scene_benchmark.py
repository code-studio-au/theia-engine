#!/usr/bin/env python3
"""Build and run Theia's procedural or cooked scene benchmark; export JSON/CSV/HTML."""
import argparse
from contextlib import nullcontext
import csv
import hashlib
import html
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from headless_display import owned_display

ROOT=Path(__file__).resolve().parents[1]

def export(report,path):
    columns=['frame','scene_time','warmup','frame_interval_ms','cpu_draw_ms','gpu_total_ms','gpu_compute_ms','gpu_visibility_ms','gpu_resolve_ms','tracked_image_and_work_allocation_bytes']
    if report['workload']=='theia-cooked-bundle-route-v1':columns+=['active_bundle_allocation_bytes','resident_texture_count','bundle_publications','bundle_deferrals']
    def csv_rows():
        for sample in report['samples']:
            row=dict(sample);cooked=sample.get('cooked',{})
            row.update(active_bundle_allocation_bytes=cooked.get('active_bundle_allocation_bytes'),resident_texture_count=cooked.get('resident_texture_count'),bundle_publications=cooked.get('publications'),bundle_deferrals=cooked.get('deferrals'))
            yield row
    with path.with_suffix('.csv').open('w',newline='') as file:
        writer=csv.DictWriter(file,fieldnames=columns,extrasaction='ignore');writer.writeheader();writer.writerows(csv_rows())
    measured=[s for s in report['samples'] if not s['warmup']]
    rows=''.join('<tr><td>'+html.escape(key)+'</td>'+''.join(f'<td>{value[field]:.3f}</td>' for field in ['mean','p50','p95','p99','max'])+'</tr>' for key,value in report['summary_ms'].items())
    data=json.dumps(measured).replace('<','\\u003c')
    page='''<!doctype html><html lang="en"><meta charset="utf-8"><title>Theia scene benchmark</title>
<style>body{font:16px system-ui;background:#101720;color:#e2e8f0;max-width:1100px;margin:3rem auto;padding:0 1rem}td,th{text-align:right;padding:.5rem 1rem}td:first-child,th:first-child{text-align:left}canvas{width:100%;height:300px;background:#182230}select{padding:.5rem}a{color:#8ad}h1{font-size:1.8rem}</style>
<h1>TITLE</h1><p>SCENE · DEVICE</p>
<p>SCOPE</p><p>Stats overlay: OVERLAY (included in timings when enabled).</p>
<p>FIFO presentation; one frame in flight. CPU values include draw/fence/acquire/present, exclude event loop gaps. GPU timestamps include synchronization and instrumentation. Warmup excluded. These values are not an uncapped FPS score.</p>
<table><tr><th>Milliseconds</th><th>Mean</th><th>p50</th><th>p95</th><th>p99</th><th>Max</th></tr>ROWS</table>
<p><label>Frame trace <select id="metric"><option>gpu_total_ms</option><option>cpu_draw_ms</option><option>gpu_compute_ms</option><option>gpu_visibility_ms</option><option>gpu_resolve_ms</option></select></label> <span id="point"></span></p>
<canvas id="plot" width="1100" height="300"></canvas>
<p>Logical route: 60 seconds by default; the selected number of frames samples the entire route. Wall duration can differ. <a href="report.json">JSON</a> · <a href="report.csv">CSV</a></p>
<script>const samples=DATA,c=document.querySelector('#plot'),ctx=c.getContext('2d'),m=document.querySelector('#metric');function draw(){const key=m.value,max=samples.reduce((v,s)=>Math.max(v,s[key]),.001);ctx.clearRect(0,0,c.width,c.height);ctx.strokeStyle='#74cfce';ctx.beginPath();samples.forEach((s,i)=>{let x=40+i/(samples.length-1)*1040,y=270-s[key]/max*240;i?ctx.lineTo(x,y):ctx.moveTo(x,y)});ctx.stroke();ctx.fillStyle='#e2e8f0';ctx.font='14px system-ui';ctx.fillText(max.toFixed(3)+' ms',8,20);ctx.fillText('Approach → village orbit → sweep',40,293)}m.onchange=draw;c.onmousemove=e=>{let i=Math.round(Math.max(0,Math.min(1,(e.offsetX/c.clientWidth*c.width-40)/1040))*(samples.length-1)),s=samples[i];document.querySelector('#point').textContent='Frame '+s.frame+', scene '+s.scene_time.toFixed(2)+'s: '+s[m.value].toFixed(3)+' ms'};draw();</script></html>'''
    cooked=report['workload']=='theia-cooked-bundle-route-v1'
    title='Theia cooked bundle benchmark' if cooked else 'Theia procedural MMO benchmark'
    scene='One complete cooked bundle at a time' if cooked else f"{report['players']} proxies · 448 static objects"
    if cooked and report.get('launcher',{}).get('scene_selection')=='whole-world':
        title='Theia cooked whole-world benchmark'
        scene='Complete resident village and terrain in one cooked bundle'
    scope=('Opaque unlit cooked geometry and diagnostic textures. Whole-bundle replacement='+str(report['streaming'])+'. No moving proxies, skinning or gameplay authority. GPU timestamps exclude uploads queued before the frame queries; CPU/frame intervals include publication stalls. Tracked draw allocations exclude bundle resources; per-frame cooked records include active bundle bytes and publication counters.' if cooked else 'Rendering workload v1. Procedural near/far detail, face shading and visual motion. No skinning, gameplay authority or content streaming.')
    page=page.replace('TITLE',title).replace('SCENE',scene).replace('SCOPE',scope).replace('OVERLAY',str(report['overlay'])).replace('DEVICE',html.escape(report['metadata']['device'])).replace('ROWS',rows).replace('DATA',data)
    path.with_suffix('.html').write_text(page)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--players',type=int,nargs='+')
    parser.add_argument('--cooked-scene',type=Path,help='scene.json from cook_benchmark_scene.py')
    parser.add_argument('--quadrant',type=int,choices=range(4))
    parser.add_argument('--whole-world',action='store_true',help='select the complete resident world from the cooked scene manifest')
    parser.add_argument('--stream-quadrant',type=int,choices=range(4))
    parser.add_argument('--bundle-gpu-budget',type=int)
    parser.add_argument('--bundle-byte-budget',type=int)
    parser.add_argument('--texture-budget',type=int)
    parser.add_argument('--frames',type=int,default=3660)
    parser.add_argument('--warmup',type=int,default=60)
    parser.add_argument('--seconds',type=float,default=60)
    parser.add_argument('--width',type=int,default=1280);parser.add_argument('--height',type=int,default=720)
    parser.add_argument('--output',type=Path,default=ROOT/'build/benchmark')
    parser.add_argument('--gpu');parser.add_argument('--headless',action='store_true');parser.add_argument('--headless-dri-prime')
    parser.add_argument('--validation',action='store_true',help='enable actual Vulkan validation; separate from performance observations')
    parser.add_argument('--no-overlay',action='store_true',help='disable stats panel for comparable timing runs')
    parser.add_argument('--profile',choices=['debug','release'],default='release')
    parser.add_argument('--slangc',default=shutil.which('slangc'))
    parser.add_argument('--skip-build',action='store_true',help='use already built matching binaries/SPIR-V')
    args=parser.parse_args();os.chdir(ROOT)
    if args.whole_world and (not args.cooked_scene or args.quadrant is not None or args.stream_quadrant is not None):parser.error('--whole-world requires --cooked-scene and cannot combine quadrant selection/replacement')
    args.quadrant=0 if args.quadrant is None else args.quadrant
    if args.cooked_scene and args.players is not None:parser.error('Cooked routes do not include procedural proxies')
    if not args.cooked_scene and (args.stream_quadrant is not None or args.quadrant != 0 or any(v is not None for v in [args.bundle_gpu_budget,args.bundle_byte_budget,args.texture_budget])):parser.error('Quadrants and bundle budgets require --cooked-scene')
    if args.stream_quadrant==args.quadrant:parser.error('Replacement must be a different quadrant')
    if any(v is not None and v<1 for v in [args.bundle_gpu_budget,args.bundle_byte_budget,args.texture_budget]):parser.error('Budgets must be positive')
    args.players=[0] if args.cooked_scene else (args.players if args.players is not None else [100,500,1000])
    if any(p<0 or p>10000 for p in args.players) or not 3<=args.frames<=120000 or not 0<=args.warmup<args.frames-2 or not 1<=args.seconds<=600 or not 64<=args.width<=8192 or not 64<=args.height<=8192:
        parser.error('Invalid bounded population/frame/warmup/duration/extent')
    if args.headless_dri_prime and not args.headless:parser.error('--headless-dri-prime requires --headless')
    args.output=args.output.resolve();args.output.mkdir(parents=True,exist_ok=True)
    (args.output/'suite.json').unlink(missing_ok=True)
    if not args.skip_build:
        if not args.slangc:parser.error('Put slangc on PATH or supply --slangc /path/to/slangc')
        with (args.output/'build.log').open('w') as log:
            subprocess.run([sys.executable,'tools/compile_render_shaders.py','--slangc',args.slangc],stdout=log,stderr=subprocess.STDOUT,check=True)
            command=['cargo','build','--locked','--manifest-path','apps/theia-render-lab/Cargo.toml']
            if args.profile=='release':command+=['--release']
            subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True)
    exe=ROOT/'apps/theia-render-lab/target'/args.profile/('theia-render-lab.exe' if os.name=='nt' else 'theia-render-lab')
    provenance={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [exe,*sorted((ROOT/'build/shaders').glob('*.spv' if args.cooked_scene else 'benchmark*.spv')),ROOT/'build/shaders/resolve.mesh.spv']}
    bundles=None
    if args.cooked_scene:
        scene=json.loads(args.cooked_scene.read_bytes())
        if not scene.get('passed') or not scene.get('cooked') or len(scene['quadrants'])!=4:parser.error('Incomplete cooked scene manifest')
        bundles=[str(Path(q['cook']['bundle']['path']).resolve()) for q in scene['quadrants']]
        if args.whole_world:
            if 'whole_world' not in scene:parser.error('Cook this manifest with --whole-world first')
            selected_bundle=str(Path(scene['whole_world']['cook']['bundle']['path']).resolve())
        else:selected_bundle=bundles[args.quadrant]
    context=owned_display(args.output/'display',args.headless_dri_prime) if args.headless else nullcontext((os.environ.copy(),os.environ.copy(),None))
    results=[]
    with context as (env,_,display):
        for number,players in enumerate(args.players):
            selection='whole-world' if args.whole_world else f'quadrant-{args.quadrant}'
            output=args.output/(f'cooked-{selection}' if bundles else f'{number:02d}-{players}-players');output.mkdir(parents=True,exist_ok=True)
            report_path=output/'report.json'
            # An old successful report must not survive a failed rerun of this owned output.
            for suffix in ['.json','.csv','.html']:report_path.with_suffix(suffix).unlink(missing_ok=True)
            mode=['--benchmark-cooked','--bundle',selected_bundle] if bundles else ['--benchmark','--benchmark-players',str(players)]
            command=[str(exe),*mode,'--frames',str(args.frames),'--benchmark-warmup',str(args.warmup),
                '--benchmark-seconds',str(args.seconds),'--benchmark-width',str(args.width),'--benchmark-height',str(args.height),'--benchmark-report',str(report_path)]
            if bundles and args.stream_quadrant is not None:command+=['--stream-bundle',bundles[args.stream_quadrant]]
            for flag,value in [('--bundle-gpu-budget',args.bundle_gpu_budget),('--bundle-byte-budget',args.bundle_byte_budget),('--texture-budget',args.texture_budget)]:
                if value is not None:command+=[flag,str(value)]
            if args.gpu:command+=['--gpu',args.gpu]
            if args.no_overlay:command+=['--benchmark-no-overlay']
            if not args.validation:command+=['--no-validation']
            print(f'Running {"cooked "+selection if bundles else str(players)+" visual proxies"}, {args.frames} frames ({args.profile}, validation={args.validation})',flush=True)
            try:
                with (output/'run.log').open('w') as log:
                    subprocess.run(command,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=max(120,args.frames/10))
            except subprocess.SubprocessError:
                report_path.unlink(missing_ok=True)
                raise
            text=(output/'run.log').read_text()
            if args.validation and 'Validation totals (including teardown): 0 warnings, 0 errors' not in text:raise RuntimeError('Missing clean validation evidence')
            report=json.loads(report_path.read_bytes());report['launcher']={'profile':args.profile,'display':display,'provenance':provenance,'command':command}
            if bundles:
                report['launcher'].update(scene_selection=selection,source_manifest=str(args.cooked_scene.resolve()),source_manifest_sha256=hashlib.sha256(args.cooked_scene.read_bytes()).hexdigest())
            report_path.write_text(json.dumps(report,indent=2)+'\n');export(report,report_path)
            results.append({'workload':report['workload'],'players':players,'report':str(report_path),'summary_ms':report['summary_ms']})
            print('Report:',report_path.with_suffix('.html'),flush=True)
    (args.output/'suite.json').write_text(json.dumps(results,indent=2)+'\n')

if __name__=='__main__':main()
