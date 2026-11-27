#!/usr/bin/env python3
"""Build and run Theia's procedural, resident or streamed scene benchmark; export JSON/CSV/HTML."""
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
    streamed=report['workload']=='theia-streamed-mmo-v1'
    columns=['frame','scene_time','warmup','frame_interval_ms','cpu_draw_ms','gpu_total_ms','gpu_compute_ms','gpu_visibility_ms','gpu_resolve_ms','tracked_image_and_work_allocation_bytes']
    if report['metadata'].get('frame_pacing'):columns+=['frame_pacing']
    if report['workload'] in ('theia-cooked-bundle-route-v1','theia-cooked-mmo-v1'):columns+=['active_bundle_allocation_bytes','resident_texture_count','bundle_publications','bundle_deferrals']
    if streamed:columns+=['world_epoch','world_selected','world_publications','world_evictions','world_deferrals','world_cpu_unique_bytes','world_graph_gpu_bytes','world_pending_gpu_bytes','world_fixed_gpu_bytes','world_total_gpu_bytes','world_image_bytes','world_pending_image_bytes','world_selected_images','world_owned_chunks','world_route_cycle']
    def csv_rows():
        for sample in report['samples']:
            row=dict(sample);cooked=sample.get('cooked',{})
            if 'frame_pacing' in row:row['frame_pacing']=json.dumps(row['frame_pacing'],separators=(',',':'))
            row.update(active_bundle_allocation_bytes=cooked.get('active_bundle_allocation_bytes'),resident_texture_count=cooked.get('resident_texture_count'),bundle_publications=cooked.get('publications'),bundle_deferrals=cooked.get('deferrals'))
            if streamed:
                world=sample['world'];pending=world.get('pending') or {}
                row.update(world_epoch=world['epoch'],world_selected=json.dumps(world['selected'],separators=(',',':')),
                    world_publications=world['publications'],world_evictions=world['evictions'],world_deferrals=world['deferrals'],
                    world_cpu_unique_bytes=world['cpu_unique_bytes'],world_graph_gpu_bytes=world['physical_bytes'],
                    world_pending_gpu_bytes=pending.get('pending_gpu_bytes',0),world_fixed_gpu_bytes=world['fixed_gpu_bytes'],
                    world_total_gpu_bytes=world['total_gpu_bytes'],world_image_bytes=world['image_bytes'],
                    world_pending_image_bytes=pending.get('pending_image_bytes',0),world_selected_images=world['selected_images'],
                    world_owned_chunks=world['owned_chunks'],world_route_cycle=world['route_cycle'])
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
<p>PRESENTATION. CPU values include draw/fence/acquire/present, exclude event loop gaps. GPU timestamps include synchronization and instrumentation. Warmup excluded. These values are not an uncapped FPS score.</p>
<table><tr><th>Milliseconds</th><th>Mean</th><th>p50</th><th>p95</th><th>p99</th><th>Max</th></tr>ROWS</table>
<p><label>Frame trace <select id="metric"><option>gpu_total_ms</option><option>cpu_draw_ms</option><option>gpu_compute_ms</option><option>gpu_visibility_ms</option><option>gpu_resolve_ms</option>WORLD_METRICS</select></label> <span id="point"></span></p>
<canvas id="plot" width="1100" height="300"></canvas>
<p>Logical route: 60 seconds by default; the selected number of frames samples the entire route. Wall duration can differ. <a href="report.json">JSON</a> · <a href="report.csv">CSV</a></p>
<script>const samples=DATA,c=document.querySelector('#plot'),ctx=c.getContext('2d'),m=document.querySelector('#metric');
function value(s,key){const v=key.split('.').reduce((row,part)=>row[part],s);return key.endsWith('_bytes')?v/1048576:v}
function units(key){return key.endsWith('_bytes')?'MiB':key.endsWith('_ms')?'ms':'count'}
function draw(){const key=m.value,max=samples.reduce((v,s)=>Math.max(v,value(s,key)),.001);ctx.clearRect(0,0,c.width,c.height);ctx.strokeStyle='#74cfce';ctx.beginPath();samples.forEach((s,i)=>{let x=40+i/(samples.length-1)*1040,y=270-value(s,key)/max*240;i?ctx.lineTo(x,y):ctx.moveTo(x,y)});ctx.stroke();ctx.fillStyle='#e2e8f0';ctx.font='14px system-ui';ctx.fillText(max.toFixed(3)+' '+units(key),8,20);ctx.fillText('Approach → village orbit → sweep',40,293)}
m.onchange=draw;c.onmousemove=e=>{let i=Math.round(Math.max(0,Math.min(1,(e.offsetX/c.clientWidth*c.width-40)/1040))*(samples.length-1)),s=samples[i];document.querySelector('#point').textContent='Frame '+s.frame+', scene '+s.scene_time.toFixed(2)+'s: '+value(s,m.value).toFixed(3)+' '+units(m.value)+(s.world?' · epoch '+s.world.epoch+' · selected '+s.world.selected.join('/'):'')};draw();</script></html>'''
    mixed=report['workload']=='theia-cooked-mmo-v1'
    cooked=report['workload'] in ('theia-cooked-bundle-route-v1','theia-cooked-mmo-v1')
    title='Theia cooked bundle benchmark' if cooked else 'Theia procedural MMO benchmark'
    scene='One complete cooked bundle at a time' if cooked else f"{report['players']} proxies · 448 static objects"
    if cooked and report.get('launcher',{}).get('scene_selection')=='whole-world':
        title='Theia cooked whole-world benchmark'
        scene='Complete resident village and terrain in one cooked bundle'
    scope=('Opaque unlit cooked geometry and diagnostic textures. Whole-bundle replacement='+str(report['streaming'])+'. No moving proxies, skinning or gameplay authority. GPU timestamps exclude uploads queued before the frame queries; CPU/frame intervals include publication stalls. Tracked draw allocations exclude bundle resources; per-frame cooked records include active bundle bytes and publication counters.' if cooked else 'Rendering workload v1. Procedural near/far detail, face shading and visual motion. No skinning, gameplay authority or content streaming.')
    if mixed:
        title='Theia cooked MMO visual benchmark'
        scene=f"Complete resident cooked world · {report['players']} GPU visual proxies"
        scope='Opaque cooked world and GPU proxy motion/detail share visibility and depth. No skinning, gameplay authority, collision or networking. Draw allocations include both work buffers; active bundle metadata retains its separate scope.'
    if streamed:
        title='Theia streamed MMO visual benchmark'
        scene=f"Four camera-streamed regions · {report['players']} GPU visual proxies"
        scope='Protected coarse coverage and complete fine publication share visibility/depth with GPU-grounded proxies. GPU frame intervals exclude earlier uploads/rebuilds; CPU draw wall includes synchronous renderer costs. Asset memory includes current/pending graphs and the terrain table; draw memory covers work/offscreen targets. Motion starts at frame zero while warmup filters summaries. No gameplay authority, skinning or production streaming claim.'
    metrics=''.join('<option value="'+key+'">'+label+'</option>' for key,label in [('world.total_gpu_bytes','Asset GPU MiB'),('world.cpu_unique_bytes','CPU encoded MiB'),('world.publications','Fine publications'),('world.evictions','Fine retirements'),('world.deferrals','Budget deferrals'),('world.epoch','World epoch')]) if streamed else ''
    if report['metadata'].get('frame_pacing'):
        from benchmark_clock import verify
        clock=verify(report,report['metadata']['frame_pacing']['target_hz'])
        scope+=' Requested %d Hz absolute frame schedule: p95 lateness %.3f ms, maximum %.3f ms, %d frames at least one period late. No skipped frames or rebasing; CPU work excludes pacing sleep. Host intervals include the requested cap.'%(clock['target_hz'],clock['p95_lateness_ms'],clock['max_lateness_ms'],clock['frames_at_least_one_period_late'])
        metrics+='<option value="frame_pacing.lateness_ms">Frame start lateness ms</option>'
    page=page.replace('PRESENTATION',html.escape(report['presentation']))
    page=page.replace('WORLD_METRICS',metrics)
    page=page.replace('TITLE',title).replace('SCENE',scene).replace('SCOPE',scope).replace('OVERLAY',str(report['overlay'])).replace('DEVICE',html.escape(report['metadata']['device'])).replace('ROWS',rows).replace('DATA',data)
    path.with_suffix('.html').write_text(page)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--players',type=int,nargs='+')
    parser.add_argument('--cooked-scene',type=Path,help='scene.json from cook_benchmark_scene.py')
    parser.add_argument('--quadrant',type=int,choices=range(4))
    parser.add_argument('--streamed-world',action='store_true',help='camera-streamed fine/coarse world with GPU proxies; auto-cook standard assets when --cooked-scene is omitted')
    parser.add_argument('--route-frames',type=int,help='streamed camera period; defaults to --frames')
    parser.add_argument('--fine-slots',type=int,choices=range(1,5),help='streamed fine residency slots (default 2)')
    parser.add_argument('--mixed-world',action='store_true',help='add GPU visual proxies to the complete resident world')
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
    parser.add_argument('--present-mode',choices=['fifo','immediate','mailbox'],default='fifo',help='exact requested swapchain mode; unsupported modes fail')
    parser.add_argument('--target-hz',type=int,help='optional 1..240 Hz absolute frame schedule; records lateness without skipping frames')
    parser.add_argument('--validation',action='store_true',help='enable actual Vulkan validation; separate from performance observations')
    parser.add_argument('--no-overlay',action='store_true',help='disable stats panel for comparable timing runs')
    parser.add_argument('--profile',choices=['debug','release'],default='release')
    parser.add_argument('--slangc',default=shutil.which('slangc'))
    parser.add_argument('--skip-build',action='store_true',help='use already built matching binaries/SPIR-V')
    args=parser.parse_args();os.chdir(ROOT)
    if args.target_hz is not None and not 1<=args.target_hz<=240:parser.error('Target Hz must be 1..240')
    if args.streamed_world and (args.mixed_world or args.whole_world or args.quadrant is not None or args.stream_quadrant is not None):parser.error('--streamed-world cannot combine resident-world or quadrant/replacement selection')
    if args.streamed_world and args.seconds!=60:parser.error('Streamed routes currently use 60 logical seconds per camera period')
    if not args.streamed_world and (args.route_frames is not None or args.fine_slots is not None):parser.error('--route-frames and --fine-slots require --streamed-world')
    if args.route_frames is not None and not 3<=args.route_frames<=120000:parser.error('Camera period must be 3..120000 frames')
    if args.mixed_world and (not args.cooked_scene or not args.whole_world):parser.error('--mixed-world requires --cooked-scene and --whole-world')
    if args.whole_world and (not args.cooked_scene or args.quadrant is not None or args.stream_quadrant is not None):parser.error('--whole-world requires --cooked-scene and cannot combine quadrant selection/replacement')
    args.quadrant=0 if args.quadrant is None else args.quadrant
    if args.cooked_scene and not args.mixed_world and not args.streamed_world and args.players is not None:parser.error('Cooked routes do not include procedural proxies')
    if not args.cooked_scene and not args.streamed_world and (args.stream_quadrant is not None or args.quadrant != 0 or any(v is not None for v in [args.bundle_gpu_budget,args.bundle_byte_budget,args.texture_budget])):parser.error('Quadrants and bundle budgets require --cooked-scene')
    if args.stream_quadrant==args.quadrant:parser.error('Replacement must be a different quadrant')
    if any(v is not None and v<1 for v in [args.bundle_gpu_budget,args.bundle_byte_budget,args.texture_budget]):parser.error('Budgets must be positive')
    args.players=(args.players if args.players is not None else [10000]) if args.streamed_world else [0] if args.cooked_scene and not args.mixed_world else (args.players if args.players is not None else [100,500,1000])
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
    if args.streamed_world and args.cooked_scene is None:
        from prepare_streamed_benchmark import prepare
        print('Preparing the eight streamed-world asset graphs',flush=True)
        args.cooked_scene=prepare(args.output/'assets',args.skip_build)
    exe=ROOT/'apps/theia-render-lab/target'/args.profile/('theia-render-lab.exe' if os.name=='nt' else 'theia-render-lab')
    provenance={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [exe,*sorted((ROOT/'build/shaders').glob('*.spv' if args.cooked_scene else 'benchmark*.spv')),ROOT/'build/shaders/resolve.mesh.spv']}
    bundles=None
    if args.cooked_scene:
        scene=json.loads(args.cooked_scene.read_bytes())
        if not scene.get('passed') or not scene.get('cooked') or len(scene['quadrants'])!=4:parser.error('Incomplete cooked scene manifest')
        bundles=[str(Path(q['cook']['bundle']['path']).resolve()) for q in scene['quadrants']]
        if args.streamed_world:
            if len(scene.get('fallbacks',[]))!=4:parser.error('Streamed worlds need four fine quadrants and four edge-compatible fallbacks')
            selected_bundle=None
        elif args.whole_world:
            if 'whole_world' not in scene:parser.error('Cook this manifest with --whole-world first')
            selected_bundle=str(Path(scene['whole_world']['cook']['bundle']['path']).resolve())
        else:selected_bundle=bundles[args.quadrant]
    context=owned_display(args.output/'display',args.headless_dri_prime) if args.headless else nullcontext((os.environ.copy(),os.environ.copy(),None))
    results=[]
    with context as (env,_,display):
        for number,players in enumerate(args.players):
            selection='streamed-world' if args.streamed_world else 'whole-world' if args.whole_world else f'quadrant-{args.quadrant}'
            output=args.output/(f'{number:02d}-streamed-{players}-proxies' if args.streamed_world else f'{number:02d}-mixed-{players}-players' if args.mixed_world else f'cooked-{selection}' if bundles else f'{number:02d}-{players}-players');output.mkdir(parents=True,exist_ok=True)
            report_path=output/'report.json'
            # An old successful report must not survive a failed rerun of this owned output.
            for suffix in ['.json','.csv','.html']:report_path.with_suffix(suffix).unlink(missing_ok=True)
            (output/'camera.json').unlink(missing_ok=True)
            if args.streamed_world:
                mode=['--world-camera-test',str(args.cooked_scene.resolve()),'--world-camera-crowd',str(players),'--world-camera-timings',str(report_path),
                    '--world-camera-report',str(output/'camera.json'),'--world-camera-warmup',str(args.warmup),
                    '--world-camera-period',str(args.route_frames or args.frames),'--world-camera-slots',str(args.fine_slots or 2)]
                command=[str(exe),*mode,'--frames',str(args.frames),'--benchmark-width',str(args.width),'--benchmark-height',str(args.height)]
                if not args.no_overlay:command+=['--world-camera-hud']
            else:
                mode=['--benchmark-mixed','--bundle',selected_bundle,'--benchmark-players',str(players)] if args.mixed_world else ['--benchmark-cooked','--bundle',selected_bundle] if bundles else ['--benchmark','--benchmark-players',str(players)]
                command=[str(exe),*mode,'--frames',str(args.frames),'--benchmark-warmup',str(args.warmup),
                    '--benchmark-seconds',str(args.seconds),'--benchmark-width',str(args.width),'--benchmark-height',str(args.height),'--benchmark-report',str(report_path)]
            if bundles and args.stream_quadrant is not None:command+=['--stream-bundle',bundles[args.stream_quadrant]]
            for flag,value in [('--bundle-gpu-budget',args.bundle_gpu_budget),('--bundle-byte-budget',args.bundle_byte_budget),('--texture-budget',args.texture_budget)]:
                if value is not None:command+=[flag,str(value)]
            command+=['--benchmark-present-mode',args.present_mode]
            if args.target_hz is not None:command+=['--benchmark-target-hz',str(args.target_hz)]
            if args.gpu:command+=['--gpu',args.gpu]
            if args.no_overlay and not args.streamed_world:command+=['--benchmark-no-overlay']
            if not args.validation:command+=['--no-validation']
            print(f'Running {"streamed "+str(players)+" visual proxies" if args.streamed_world else "mixed "+str(players)+" visual proxies" if args.mixed_world else "cooked "+selection if bundles else str(players)+" visual proxies"}, {args.frames} frames ({args.profile}, validation={args.validation})',flush=True)
            try:
                with (output/'run.log').open('w') as log:
                    subprocess.run(command,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=max(120,args.frames/10,2*args.frames/(args.target_hz or 240)+60))
            except subprocess.SubprocessError:
                report_path.unlink(missing_ok=True)
                raise
            text=(output/'run.log').read_text()
            if args.validation and 'Validation totals (including teardown): 0 warnings, 0 errors' not in text:raise RuntimeError('Missing clean validation evidence')
            report=json.loads(report_path.read_bytes());report['launcher']={'profile':args.profile,'display':display,'provenance':provenance,'command':command}
            if bundles:
                report['launcher'].update(scene_selection=selection,mixed_world=args.mixed_world,streamed_world=args.streamed_world,source_manifest=str(args.cooked_scene.resolve()),source_manifest_sha256=hashlib.sha256(args.cooked_scene.read_bytes()).hexdigest())
            if args.streamed_world:(output/'scene-manifest.json').write_bytes(args.cooked_scene.read_bytes())
            report_path.write_text(json.dumps(report,indent=2)+'\n');export(report,report_path)
            results.append({'workload':report['workload'],'players':players,'report':str(report_path),'summary_ms':report['summary_ms']})
            print('Report:',report_path.with_suffix('.html'),flush=True)
    (args.output/'suite.json').write_text(json.dumps(results,indent=2)+'\n')

if __name__=='__main__':main()
