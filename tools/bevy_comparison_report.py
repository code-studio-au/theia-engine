"""Standalone interactive host-frame trace with explicit comparison limits."""
import html
import json
from pathlib import Path


def write(output,result):
    traces=[];rows=[]
    for case in result['cases']:
        source=json.loads(Path(case['theia_report']).read_bytes());reference=json.loads(Path(case['bevy_report']).read_bytes())
        traces.append(dict(players=case['players'],warmup=source['warmup_frames'],theia=source['samples'],bevy=reference['samples']))
        for engine,summary in [('Theia',source['summary_ms']['frame_interval_ms']),('Bevy',reference['summary_frame_interval_ms'])]:
            rows.append('<tr><td>'+str(case['players'])+'</td><td>'+engine+'</td><td>Host interval</td>'+''.join('<td>%.3f</td>'%summary[k] for k in ['mean','p50','p95','p99','max'])+'</tr>')
        if result.get('gpu_timings'):
            for engine,summary,label in [('Theia',source['summary_ms']['gpu_total_ms'],'GPU frame'),('Bevy',reference['summary_gpu_render_graph_ms'],'GPU render graph')]:
                rows.append('<tr><td>'+str(case['players'])+'</td><td>'+engine+'</td><td>'+label+'</td>'+''.join('<td>%.3f</td>'%summary[k] for k in ['mean','p50','p95','p99','max'])+'</tr>')
    choices=''.join('<option value="%d">%d proxies</option>'%(i,case['players']) for i,case in enumerate(result['cases']))
    data=json.dumps(traces,separators=(',',':')).replace('<','\\u003c')
    page='''<!doctype html><html lang="en"><meta charset="utf-8"><title>Theia / Bevy visual benchmark</title>
<style>body{font:16px system-ui;background:#111923;color:#e5edf5;max-width:1100px;margin:3rem auto;padding:0 1rem}table{border-collapse:collapse}td,th{padding:.5rem;text-align:right}td:nth-child(2){text-align:left}canvas{width:100%;height:320px;background:#1b2835}select{padding:.5rem}a{color:#9cd}</style>
<h1>Theia / Bevy visual benchmark</h1><p>SCOPE</p><p>CLOCK_SCOPE</p>
<p><strong>PRESENT_MODE presentation: host-frame intervals can remain compositor-limited; these are not equivalent GPU timings.</strong> Theia streams assets and simulates proxies on the GPU. Bevy preloads all eight graphs and updates proxy presentation through CPU ECS. The recorded logical camera/selection route is shared. OVERLAY_SCOPE</p>
<p>Bevy CPU main-schedule time excludes later extraction/submission; Theia CPU draw time includes fence/acquire/present. These CPU figures have different scopes and are not ranked here. Validation requested: VALIDATION.</p><p>GPU_SCOPE</p>
<table><tr><th>Proxies</th><th>Engine</th><th>Metric</th><th>Mean ms</th><th>p50</th><th>p95</th><th>p99</th><th>Max</th></tr>ROWS</table>
<p><select id="population">CHOICES</select> <select id="metric"><option value="host">Host interval</option>GPU_CHOICE</select> <span style="color:#6ed7ca">Theia</span> / <span style="color:#ffb777">Bevy</span> · measured frames only · <span id="point"></span></p><canvas id="plot" width="1100" height="320"></canvas>
<p><a href="comparison.json">Configuration, provenance and report paths</a>. Bevy JSON/CSV files and Theia JSON/CSV/HTML files remain in their population directories. Frozen screenshot captures are separate from timing runs.</p>
<script>const data=DATA,c=document.querySelector('#plot'),ctx=c.getContext('2d'),select=document.querySelector('#population'),metric=document.querySelector('#metric');
function value(s,key){return metric.value==='gpu'?s[key==='theia'?'gpu_total_ms':'gpu_render_graph_ms']:s.frame_interval_ms}
function measured(t,key){return t[key].slice(t.warmup)}
function draw(){const t=data[+select.value];const max=['theia','bevy'].reduce((max,key)=>measured(t,key).reduce((max,s)=>Math.max(max,value(s,key)),max),.001);ctx.clearRect(0,0,c.width,c.height);['theia','bevy'].forEach((key,n)=>{ctx.beginPath();ctx.strokeStyle=n?'#ffb777':'#6ed7ca';measured(t,key).forEach((s,i,a)=>{let x=40+1040*i/(a.length-1),y=280-value(s,key)/max*240;i?ctx.lineTo(x,y):ctx.moveTo(x,y)});ctx.stroke()});ctx.fillStyle='#e5edf5';ctx.fillText(max.toFixed(3)+' ms',8,20)}
select.onchange=draw;metric.onchange=draw;c.onmousemove=e=>{const t=data[+select.value],a=measured(t,'theia'),b=measured(t,'bevy'),i=Math.round(Math.max(0,Math.min(1,(e.offsetX/c.clientWidth*c.width-40)/1040))*(a.length-1));document.querySelector('#point').textContent='Frame '+a[i].frame+' · logical '+a[i].scene_time.toFixed(2)+'s · Theia '+value(a[i],'theia').toFixed(3)+' ms · Bevy '+value(b[i],'bevy').toFixed(3)+' ms'};draw();</script></html>'''
    page=page.replace('GPU_CHOICE','<option value="gpu">GPU rendering intervals</option>' if result.get('gpu_timings') else '')
    page=page.replace('GPU_SCOPE',('GPU intervals: Theia compute/visibility/resolve frame timestamps versus Bevy render-graph encoder timestamps (including renderer compute, raster, postprocess and enabled UI). Uploads before these ranges, host presentation work and readback are excluded. These are different renderer pipelines; no pass-by-pass equivalence or total-engine ranking. Bevy serially waits for timing readback after rendering; this affects its host cadence.' if result.get('gpu_timings') else 'Bevy GPU timestamps were disabled; no GPU comparison is available.'))
    page=page.replace('PRESENT_MODE',html.escape(result.get('present_mode','fifo').upper()))
    if result.get('target_hz'):
        clocks=[]
        for case in result['cases']:
            for engine in ('theia','bevy'):
                proof=case['clock_proof'][engine]
                clocks.append('%d proxies / %s: p95 lateness %.3f ms, max %.3f ms, %d frames at least one period late'%(case['players'],engine,proof['p95_lateness_ms'],proof['max_lateness_ms'],proof['frames_at_least_one_period_late']))
        clock='Requested %d Hz absolute frame schedule; no skipped frames or rebasing. '%result['target_hz']+'; '.join(clocks)+'. These are frame-start deadlines; I/O and presentation completion are not measured by this clock.'
    else:
        clock='No host frame schedule requested; logical camera steps can have different wall-clock spacing in each renderer.'
    page=page.replace('CLOCK_SCOPE',html.escape(clock))
    page=page.replace('OVERLAY_SCOPE',('Both stats displays enabled: same panel footprint/cadence, different fonts and available metrics. Theia uses its GPU debug font; Bevy uses native UI text. Bevy GPU milliseconds appear only when GPU timing is enabled.' if result.get('stats_overlay') else 'Both overlays are disabled.'))
    page=page.replace('SCOPE',html.escape(result['scope'])).replace('VALIDATION',str(result['validation_requested'])).replace('ROWS',''.join(rows)).replace('CHOICES',choices).replace('DATA',data)
    (output/'comparison.html').write_text(page)
