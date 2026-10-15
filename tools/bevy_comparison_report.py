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
            rows.append('<tr><td>'+str(case['players'])+'</td><td>'+engine+'</td>'+''.join('<td>%.3f</td>'%summary[k] for k in ['mean','p50','p95','p99','max'])+'</tr>')
    choices=''.join('<option value="%d">%d proxies</option>'%(i,case['players']) for i,case in enumerate(result['cases']))
    data=json.dumps(traces,separators=(',',':')).replace('<','\\u003c')
    page='''<!doctype html><html lang="en"><meta charset="utf-8"><title>Theia / Bevy visual benchmark</title>
<style>body{font:16px system-ui;background:#111923;color:#e5edf5;max-width:1100px;margin:3rem auto;padding:0 1rem}table{border-collapse:collapse}td,th{padding:.5rem;text-align:right}td:nth-child(2){text-align:left}canvas{width:100%;height:320px;background:#1b2835}select{padding:.5rem}a{color:#9cd}</style>
<h1>Theia / Bevy visual benchmark</h1><p>SCOPE</p>
<p><strong>FIFO presentation: these are host-frame intervals, not uncapped FPS or equivalent GPU timings.</strong> Theia streams assets and simulates proxies on the GPU. Bevy preloads all eight graphs and updates proxy presentation through CPU ECS. The recorded logical camera/selection route is shared. Both overlays are disabled.</p>
<p>Bevy CPU main-schedule time excludes later extraction/submission; Theia CPU draw time includes fence/acquire/present. These CPU figures have different scopes and are not ranked here. Validation requested: VALIDATION.</p>
<table><tr><th>Proxies</th><th>Engine</th><th>Mean ms</th><th>p50</th><th>p95</th><th>p99</th><th>Max</th></tr>ROWS</table>
<p><select id="population">CHOICES</select> <span style="color:#6ed7ca">Theia</span> / <span style="color:#ffb777">Bevy</span> · measured frames only · <span id="point"></span></p><canvas id="plot" width="1100" height="320"></canvas>
<p><a href="comparison.json">Configuration, provenance and report paths</a>. Bevy JSON/CSV files and Theia JSON/CSV/HTML files remain in their population directories. Frozen screenshot captures are separate from timing runs.</p>
<script>const data=DATA,c=document.querySelector('#plot'),ctx=c.getContext('2d'),select=document.querySelector('#population');
function measured(t,key){return t[key].slice(t.warmup)}
function draw(){const t=data[+select.value];const max=['theia','bevy'].reduce((max,key)=>measured(t,key).reduce((max,s)=>Math.max(max,s.frame_interval_ms),max),.001);ctx.clearRect(0,0,c.width,c.height);['theia','bevy'].forEach((key,n)=>{ctx.beginPath();ctx.strokeStyle=n?'#ffb777':'#6ed7ca';measured(t,key).forEach((s,i,a)=>{let x=40+1040*i/(a.length-1),y=280-s.frame_interval_ms/max*240;i?ctx.lineTo(x,y):ctx.moveTo(x,y)});ctx.stroke()});ctx.fillStyle='#e5edf5';ctx.fillText(max.toFixed(3)+' ms',8,20)}
select.onchange=draw;c.onmousemove=e=>{const t=data[+select.value],a=measured(t,'theia'),b=measured(t,'bevy'),i=Math.round(Math.max(0,Math.min(1,(e.offsetX/c.clientWidth*c.width-40)/1040))*(a.length-1));document.querySelector('#point').textContent='Frame '+a[i].frame+' · logical '+a[i].scene_time.toFixed(2)+'s · Theia '+a[i].frame_interval_ms.toFixed(3)+' ms · Bevy '+b[i].frame_interval_ms.toFixed(3)+' ms'};draw();</script></html>'''
    page=page.replace('SCOPE',html.escape(result['scope'])).replace('VALIDATION',str(result['validation_requested'])).replace('ROWS',''.join(rows)).replace('CHOICES',choices).replace('DATA',data)
    (output/'comparison.html').write_text(page)
