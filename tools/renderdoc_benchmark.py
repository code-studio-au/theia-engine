"""Owned qrenderdoc process: capture, independently check dense scene data, save a preview."""
import json
import math
import os
from pathlib import Path
import shlex
import struct
import subprocess
import time
import traceback
import renderdoc as rd

def success(result):
    if result!=rd.ResultCode.Succeeded:raise RuntimeError(str(result))

def walk(actions):
    for action in actions:
        yield action
        yield from walk(action.children)

import sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path.cwd() / 'tools'))
from proxy_benchmark_reference import dot, sub, cross, pixel_reference, color_reference, encode, conservative_visible

def qualify(report):
    output=Path(os.environ['THEIA_BENCHMARK_CAPTURE_OUTPUT']).resolve();output.mkdir(parents=True,exist_ok=True)
    players=int(os.environ.get('THEIA_BENCHMARK_PLAYERS','1000'))
    frame=int(os.environ.get('THEIA_BENCHMARK_CAPTURE_FRAME','90'))
    frames=181
    if not 0<=frame<frames:raise RuntimeError('Capture frame outside bounded route')
    exe=Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
    options=rd.CaptureOptions();options.apiValidation=False;options.debugOutputMute=False
    args=['--benchmark','--benchmark-players',str(players),'--frames',str(frames),'--benchmark-warmup','0',
          '--benchmark-report',str(output/'timings.json'),'--no-validation','--gpu',os.environ.get('THEIA_BENCHMARK_GPU','7900')]
    overlay=os.environ.get('THEIA_BENCHMARK_OVERLAY')=='1'
    if not overlay:args+=['--benchmark-no-overlay']
    result=rd.ExecuteAndInject(str(exe),str(Path.cwd()),shlex.join(args),[],str(output/'scene'),options,False)
    success(result.result)
    target=rd.CreateTargetControl('',result.ident,'Theia benchmark qualification',True)
    if target is None:raise RuntimeError('No capture target')
    capture=None
    try:
        target.QueueCapture(frame,1);deadline=time.monotonic()+45
        while target.Connected() and time.monotonic()<deadline:
            message=target.ReceiveMessage(None)
            if message.type==rd.TargetControlMessageType.NewCapture:capture=message.newCapture.path
        if target.Connected() or capture is None:raise RuntimeError('Capture target timed out or produced no capture')
    finally:target.Shutdown()
    cap=rd.OpenCaptureFile();controller=None
    try:
        success(cap.OpenFile(capture,'',None));result,controller=cap.OpenCapture(rd.ReplayOptions(),None);success(result)
        actions=list(walk(controller.GetRootActions()))
        draws=[a for a in actions if a.flags&rd.ActionFlags.MeshDispatch and a.flags&rd.ActionFlags.Indirect and not a.flags&rd.ActionFlags.MultiAction]
        if len(draws)!=1 or not any(a.flags&rd.ActionFlags.Dispatch for a in actions):raise RuntimeError('Missing compute/indirect mesh pipeline')
        controller.SetFrameEvent(draws[0].eventId,True)
        buffers=[b for b in controller.GetBuffers() if b.length==376160]
        if len(buffers)!=1:raise RuntimeError('Missing unique dense work buffer')
        data=controller.GetBufferData(buffers[0].resourceId,0,376160)
        count,y,z,draw_count,total,near,far,visible_players=struct.unpack_from('<8I',data)
        if y!=1 or z!=1 or draw_count!=1 or total!=players+448 or count>total:raise RuntimeError('Invalid GPU indirect work bounds')
        present=next(a for a in actions if a.flags&rd.ActionFlags.Present)
        controller.SetFrameEvent(present.eventId,True)
        textures=controller.GetTextures();color=next(t for t in textures if t.resourceId==present.copyDestination)
        reference=json.loads(subprocess.check_output([str(exe),'--benchmark-reference',str(players),str(frame),str(frames),str(color.width),str(color.height)]))
        visible=[];max_error=0.;gpu_objects=[]
        for obj in reference['objects']:
            id=obj['id'];p=struct.unpack_from('<3fI3fI',data,41824+id*32)
            error=max(abs(a-b) for a,b in zip(p[:3],obj['center']));max_error=max(max_error,error)
            if error>0.001 or p[3]!=obj['kind'] or any(abs(a-b)>1e-6 for a,b in zip(p[4:7],obj['extent'])):
                raise RuntimeError(f'GPU instance/reference mismatch {id}: {p} expected {obj}')
            # Motion has a separate bounded f32/trigonometric check. Culling/detail
            # must apply correctly to the actual generated records, including cutoff edges.
            actual_near=p[3]!=0 and dot(sub(p[:3],reference['eye']),sub(p[:3],reference['eye']))<35*35
            if bool(p[7])!=actual_near:raise RuntimeError(f'GPU detail rule differs on actual position {id}')
            if conservative_visible(reference['rows'],p[:3],p[4:7]):visible.append(id)
            gpu_objects.append((id,p[:3],p[4:7],bool(p[7])))
        ids=list(struct.unpack_from(f'<{count}I',data,32))
        if len(set(ids))!=count or sorted(ids)!=visible:raise RuntimeError('GPU compaction/culling differs from CPU reference')
        expected_near=sum(gpu_objects[id][3] for id in ids)
        if near!=expected_near or far!=count-near or visible_players!=sum(id>=448 for id in ids):raise RuntimeError('Detail/player counters differ')
        identity=next(t for t in textures if t.format.compType==rd.CompType.UInt and t.format.compCount==2)
        depth=next(t for t in textures if t.format.compType==rd.CompType.Depth)
        identities=controller.GetTextureData(identity.resourceId,rd.Subresource());depths=controller.GetTextureData(depth.resourceId,rd.Subresource())
        seen=set();covered=0;visible_set=set(ids)
        for offset in range(color.width*color.height):
            id,primitive=struct.unpack_from('<2I',identities,offset*8);d=struct.unpack_from('<f',depths,offset*4)[0]
            if not math.isfinite(d) or not 0<=d<=1:raise RuntimeError('Invalid depth pixel')
            if id:
                index=id-1
                if index not in visible_set or primitive>=(2 if index<256 else 16 if gpu_objects[index][3] else 12) or d>=1:raise RuntimeError('Invalid raster identity/detail/depth')
                seen.add(index);covered+=1
            elif d!=1:raise RuntimeError('Background has geometry depth')
        if not covered:raise RuntimeError('Scene rendered no covered pixels')
        rgb=controller.GetTextureData(color.resourceId,rd.Subresource());samples=[]
        if overlay:
            # Decode expected strings from original-run timing samples and current GPU headers.
            timings=json.loads((output/'timings.json').read_bytes());last=timings['samples'][max(0,frame-1)]
            recent=timings['samples'][max(0,frame-30):frame]
            scaled=lambda value,scale:math.floor(value*scale+.5)
            fps=scaled(1000*len(recent)/sum(s['frame_interval_ms'] for s in recent),10) if recent else 0
            cpu=scaled(last['cpu_draw_ms'],1000) if frame else 0;gpu=scaled(last['gpu_total_ms'],1000) if frame else 0
            memory=scaled(last['tracked_image_and_work_allocation_bytes']/1048576,10)
            phase=min(2,math.floor(reference['time']/60*3))+1;clock=scaled(reference['time'],10)
            lines=['THEIA BENCHMARK',f'FPS {fps/10:.1f}',f'CPU MS {cpu/1000:.3f}',f'GPU MS {gpu/1000:.3f}',
                f'VISIBLE {count}/{total}',f'PROXIES {visible_players}/{players}',f'NEAR {near} FAR {far}',f'ALLOC MIB {memory/10:.1f}',f'PHASE {phase} T {clock/10:.1f}']
            # The repo-native font table is the encoding contract, not a system font dependency.
            import re
            source=Path('shaders/render/benchmark_hud.slang').read_text()
            font=[(int(a,16),int(b)) for a,b in re.findall(r'uint2\(0x([0-9a-f]+)u,(\d+)u\)',source)]
            chars='0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ./';checks=0
            for line,text in enumerate(lines):
                for column,c in enumerate(text):
                    for row in range(7):
                        for cell in range(5):
                            index=chars.find(c)
                            bits=((font[index][0]>>(row*5))&31 if row<6 else font[index][1]) if index>=0 else 0
                            lit=bits&(1<<(4-cell))!=0
                            x=24+column*18+cell*3+1;y=24+line*24+row*3+1;offset=y*color.width+x
                            actual=list(rgb[offset*4:offset*4+3])
                            if color.format.BGRAOrder():actual.reverse()
                            expected=list(map(encode,[.7,.92,.76] if lit else [.012,.025,.04]))
                            if any(abs(a-b)>2 for a,b in zip(actual,expected)):raise RuntimeError(f'Stats glyph mismatch {line}/{column}/{row}/{cell}: {text}')
                            checks+=1
            report.update(overlay_lines=lines,overlay_glyph_pixels=checks,overlay_verified=True)
        candidates=[gpu_objects[id] for id in ids]
        for yf in [.1,.3,.5,.7,.9]:
            for xf in [.1,.3,.5,.7,.9]:
                x=int(color.width*xf);y=int(color.height*yf);offset=y*color.width+x
                if overlay and 12<=x<420 and 12<=y<252:continue
                expected_id,primitive,expected_depth=pixel_reference(reference['rows'],candidates,x,y,color.width,color.height)
                actual_id,actual_primitive=struct.unpack_from('<2I',identities,offset*8)
                actual_depth=struct.unpack_from('<f',depths,offset*4)[0]
                if actual_id!=expected_id or abs(actual_depth-expected_depth)>2e-5:raise RuntimeError(f'Ray/raster identity or depth mismatch at {x},{y}: {actual_id}/{actual_depth} expected {expected_id}/{expected_depth}')
                kind=reference['objects'][expected_id-1]['kind'] if expected_id else 0
                expected_color=list(map(encode,color_reference(expected_id,primitive,expected_depth,kind)))
                actual_color=list(rgb[offset*4:offset*4+3])
                if color.format.BGRAOrder():actual_color.reverse()
                if any(abs(a-b)>2 for a,b in zip(actual_color,expected_color)):raise RuntimeError(f'CPU/deferred colour mismatch at {x},{y}: {actual_color} expected {expected_color}')
                samples.append({'pixel':[x,y],'identity':actual_id,'primitive':actual_primitive,'depth':actual_depth,'color':actual_color})
        save=rd.TextureSave();save.resourceId=present.copyDestination;save.destType=rd.FileType.PNG
        success(controller.SaveTexture(save,str(output/'preview.png')))
        report.update(passed=True,capture=capture,players=players,frame=frame,time=reference['time'],visible=count,
            near_detail=near,far_detail=far,visible_players=visible_players,raster_objects=len(seen),covered_pixels=covered,
            instance_position_max_error=max_error,all_instances_verified=True,compaction_verified=True,image_identity_depth_verified=True,
            ray_and_color_samples=samples,overlay=overlay,extent=[color.width,color.height])
    finally:
        if controller is not None:controller.Shutdown()
        cap.Shutdown()

report={'passed':False,'renderdoc':rd.GetVersionString(),'validation_in_capture':False};code=0
try:qualify(report)
except Exception:report['error']=traceback.format_exc();code=1
finally:
    output=Path(os.environ['THEIA_BENCHMARK_CAPTURE_OUTPUT']);output.mkdir(parents=True,exist_ok=True)
    (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
os._exit(code)
