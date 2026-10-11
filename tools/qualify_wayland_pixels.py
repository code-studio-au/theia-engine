"""Offline analysis of the owned nested compositor's actual output screenshot.
The native screenshooter writes PNG; this module reads it and compares a fixed
GPU-generated indirect triangle scene against an analytical raster/color oracle.
"""
import json
import math
from pathlib import Path
import subprocess
import time


def srgb(value):
    return round(255*(12.92*value if value<=.0031308 else 1.055*value**(1/2.4)-.055))


def viewport(image, background):
    pixels=image.load()
    runs={}
    # Full clear rows identify the fixed 960px canvas without assuming compositor
    # placement/decorations. Both canvas edge columns remain clear for all counts.
    for y in range(image.height):
        start=None
        for x in range(image.width+1):
            match=x<image.width and pixels[x,y][:3]==background
            if match and start is None:start=x
            if not match and start is not None:
                if x-start==960:runs[start]=runs.get(start,0)+1
                start=None
    if not runs:raise RuntimeError('No unscaled 960px background rows')
    left=max(runs,key=runs.get)
    spans=[];start=None
    for y in range(image.height+1):
        match=y<image.height and pixels[left,y][:3]==background and pixels[left+959,y][:3]==background
        if match and start is None:start=y
        if not match and start is not None:spans.append((start,y-start));start=None
    top,height=max(spans,key=lambda v:v[1])
    if height!=540:raise RuntimeError('Canvas height does not match fixed swapchain: '+str(spans))
    return left,top,960,540


def verify(path,count):
    from PIL import Image
    with Image.open(path) as image:
        if image.size!=(1400,900):raise RuntimeError('Unexpected nested output size')
        pixels=image.load();background=tuple(srgb(v) for v in [.025,.07,.12])
        left,top,width,height=viewport(image,background)
        triangles=[]
        for offset in [-.5,.5][:count]:
            triangles.append([((x*.45+offset+1)*width/2,(y*.45+1)*height/2)
                for x,y in [(-.7,.6),(.7,.6),(0,-.6)]])
        coverage=[0,0,0];checked=0
        def cross(a,b):return a[0]*b[1]-a[1]*b[0]
        for y in range(4,height-4,8):
            for x in range(4,width-4,8):
                point=(x+.5,y+.5);expected=background;chosen=0;boundary=False
                for index,vertices in enumerate(triangles):
                    a,b,c=vertices
                    sub=lambda u,v:(u[0]-v[0],u[1]-v[1])
                    area=cross(sub(b,a),sub(c,a))
                    weights=[cross(sub(b,point),sub(c,point))/area,
                             cross(sub(c,point),sub(a,point))/area,
                             cross(sub(a,point),sub(b,point))/area]
                    for u,v in [(a,b),(b,c),(c,a)]:
                        if abs(cross(sub(v,u),sub(point,u)))/math.dist(u,v)<=1:
                            boundary=True
                    if min(weights)>=0:
                        expected=tuple(srgb(value) for value in weights);chosen=index+1
                if boundary:continue
                actual=pixels[left+x,top+y][:3]
                if any(abs(a-b)>2 for a,b in zip(actual,expected)):
                    raise RuntimeError(f'Composited pixel mismatch count {count} at {(x,y)}: {actual}, expected {expected}')
                coverage[chosen]+=1;checked+=1
        if coverage[0]<100 or any(coverage[i+1]<10 for i in range(count)):
            raise RuntimeError('Insufficient visible triangle/background coverage')
        return {'work_items':count,'output_png':str(path),'viewport':[left,top,width,height],
            'sampled_pixels':checked,'background_left_right_counts':coverage,'pixel_reference_verified':True}


def qualify(weston_root,client_env,output):
    output=output/'pixels';output.mkdir(parents=True,exist_ok=True)
    executable=Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
    shooter_env=client_env.copy();shooter_env['XDG_PICTURES_DIR']=str(output)
    libs=weston_root/'usr/lib/x86_64-linux-gnu'
    shooter_env['LD_LIBRARY_PATH']=str(libs)+':'+str(libs/'weston')
    reports=[]
    for count in range(3):
        process=None;log_path=output/f'client-{count}.log'
        with log_path.open('w') as log:
            try:
                process=subprocess.Popen([str(executable),'--gpu','7900','--work-items',str(count),'--frames','300'],
                    env=client_env,stdout=log,stderr=subprocess.STDOUT)
                deadline=time.monotonic()+10
                while 'Swapchain generation 2 ready' not in log_path.read_text():
                    if time.monotonic()>=deadline or process.poll() is not None:
                        raise RuntimeError('Pixel client failed to initialize: '+log_path.read_text())
                    time.sleep(.02)
                time.sleep(.8)
                before=set(output.glob('*.png'))
                with (output/f'shooter-{count}.log').open('w') as shot_log:
                    subprocess.run([str(weston_root/'usr/bin/weston-screenshooter')],env=shooter_env,
                        stdout=shot_log,stderr=subprocess.STDOUT,check=True,timeout=10)
                paths=set(output.glob('*.png'))-before
                if len(paths)!=1:raise RuntimeError('Screenshooter did not produce exactly one owned PNG')
                report=verify(paths.pop(),count)
                if process.wait(timeout=30)!=0:raise RuntimeError('Pixel client failed')
                text=log_path.read_text()
                for required in ['Native surface created (Wayland)','Presented 300 frames',
                    'Validation totals (including teardown): 0 warnings, 0 errors']:
                    if required not in text:raise RuntimeError('Missing '+required)
                reports.append(report)
                print('Qualified composited Wayland pixels:',count,report['sampled_pixels'],flush=True)
            finally:
                if process is not None and process.poll() is None:
                    process.terminate()
                    try:process.wait(timeout=3)
                    except subprocess.TimeoutExpired:process.kill();process.wait()
    (output/'report.json').write_text(json.dumps({'compositor_screenshot':True,'renderdoc_replay':False,'cases':reports},indent=2)+'\n')
