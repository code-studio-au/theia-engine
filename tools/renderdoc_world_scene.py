"""Replay oracle for four complete resident world chunks and buffer arrays."""
import json
import hashlib
from collections import Counter
import math
import os
from pathlib import Path
import shlex
import struct
import subprocess
import sys
sys.dont_write_bytecode = True
import time
import traceback
import renderdoc as rd
sys.path.insert(0,str(Path.cwd()/'tools'))
from cooked_benchmark_reference import decode,ray,dot,material_pixels,material_factors
from proxy_benchmark_reference import encode
import world_crowd_reference as crowd_reference


def require(value,message):
    if not value:raise RuntimeError(message)


def success(value):
    require(value==rd.ResultCode.Succeeded,str(value))


def walk(actions):
    for action in actions:
        yield action
        yield from walk(action.children)


def linear(byte):
    value=byte/255.
    return value/12.92 if value<=.04045 else ((value+.055)/1.055)**2.4


def qualify(report):
    out=Path(os.environ['THEIA_WORLD_SCENE_OUTPUT']).resolve();out.mkdir(parents=True,exist_ok=True)
    scene_path=Path(os.environ['THEIA_WORLD_SCENE'])
    scene=json.loads(scene_path.read_bytes());cooks=[e['cook'] for e in scene['quadrants']]
    camera=os.environ.get('THEIA_WORLD_SCENE_CAMERA')=='1'
    crowd=int(os.environ['THEIA_WORLD_SCENE_CROWD']) if 'THEIA_WORLD_SCENE_CROWD' in os.environ else None
    require(crowd is None or (camera and 0<=crowd<=10000),'Crowd requires camera profile')
    covered=os.environ.get('THEIA_WORLD_SCENE_COVERED') or ('[4,5,6,7]' if camera else None)
    selected=json.loads(covered) if covered else list(range(4))
    if covered:
        require(len(selected)==4 and all(v in (i,i+4) for i,v in enumerate(selected)), 'Invalid protected selection')
        cooks=[scene['quadrants' if value<4 else 'fallbacks'][i]['cook'] for i,value in enumerate(selected)]
        inventory_cooks=[entry['cook'] for entry in scene['fallbacks']]+[scene['quadrants'][i]['cook'] for i,value in enumerate(selected) if value<4]
    else:inventory_cooks=cooks
    require(len(cooks)==4,'Needs four complete chunks')
    models=[decode(cook) for cook in cooks]
    inventory_models=[decode(cook) for cook in inventory_cooks]
    pixel_tables=[material_pixels(cook,model[3]) for cook,model in zip(cooks,models)]
    factors=[material_factors(cook) for cook in cooks]
    view=int(os.environ.get('THEIA_WORLD_SCENE_VIEW','2'))
    frame=int(os.environ.get('THEIA_WORLD_SCENE_FRAME','10'))
    frames=int(os.environ.get('THEIA_WORLD_SCENE_FRAMES','90'))
    require(0<=frame<frames<=2000,'Invalid capture frame/run length')
    reload=os.environ.get('THEIA_WORLD_SCENE_RELOAD')=='1'
    retire=os.environ.get('THEIA_WORLD_SCENE_RETIRE')=='1' or reload
    exe=Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
    args=['--world-camera-test' if camera else '--world-covered-test' if covered else '--world-scene-test',str(scene_path),'--frames',str(frames),'--work-items',str(view),'--no-validation','--gpu','7900']
    if camera:
        args=args[:args.index('--work-items')]+args[args.index('--work-items')+2:]
        args+=['--world-camera-period',os.environ.get('THEIA_WORLD_SCENE_PERIOD',str(frames)),
            '--world-camera-slots',os.environ.get('THEIA_WORLD_SCENE_SLOTS','2'),'--world-camera-report',str(out/'camera.json')]
    if crowd is not None:args+=['--world-camera-crowd',str(crowd)]
    timed=os.environ.get('THEIA_WORLD_SCENE_TIMINGS')=='1'
    if timed:
        require(crowd is not None,'Timings require world crowd profile')
        args+=['--world-camera-timings',str(out/'timings.json'),'--world-camera-warmup',os.environ.get('THEIA_WORLD_SCENE_WARMUP','10')]
    if retire:
        require(covered,'Retirement requires protected selection')
        args+=['--world-covered-reload-test' if reload else '--world-covered-retire-test','--world-covered-report',str(out/'retirement.json')]
    if os.environ.get('THEIA_WORLD_SCENE_CAPS'):
        cpu,gpu,images=json.loads(os.environ['THEIA_WORLD_SCENE_CAPS'])
        args+=['--bundle-byte-budget',str(cpu),'--bundle-gpu-budget',str(gpu),'--texture-budget',str(images)]
    if os.environ.get('THEIA_WORLD_SCENE_SHADERS'):
        args+=['--shader-dir',os.environ['THEIA_WORLD_SCENE_SHADERS']]
    options=rd.CaptureOptions();options.apiValidation=False;options.debugOutputMute=False
    options.refAllResources=bool(covered)
    capture=os.environ.get('THEIA_WORLD_SCENE_CAPTURE')
    if not capture:
        launched=rd.ExecuteAndInject(str(exe),str(Path.cwd()),shlex.join(args),[],str(out/'world'),options,False)
        success(launched.result)
        target=rd.CreateTargetControl('',launched.ident,'Theia four-chunk qualification',True)
        require(target is not None,'Missing target control')
        capture=None
        try:
            target.QueueCapture(frame,1)
            deadline=time.monotonic()+60
            while target.Connected() and time.monotonic()<deadline:
                message=target.ReceiveMessage(None)
                if message.type==rd.TargetControlMessageType.NewCapture:capture=message.newCapture.path
            require(not target.Connected() and capture,'Missing capture or target timeout')
        finally:target.Shutdown()
    retirement=None
    if retire:
        retirement=json.loads((out/'retirement.json').read_bytes())
        require(retirement['completed'],'Retirement run incomplete')
        events=[e for e in retirement['history'] if e['frame']<=frame]
        require(events and events[-1]['selected']==selected,'Captured selection differs from published CPU history')
    camera_report=None;camera_state=None;staging_models=[]
    if camera:
        camera_report=json.loads((out/'camera.json').read_bytes())
        require(camera_report['completed'],'Camera run incomplete')
        events=[e for e in camera_report['history'] if e['frame']<=frame]
        require(events,'Missing camera state at captured frame')
        camera_state=events[-1];selected=camera_state['selected']
        require(len(selected)==4 and all(v in (i,i+4) for i,v in enumerate(selected)),'Bad committed camera selection')
        cooks=[scene['quadrants' if value<4 else 'fallbacks'][i]['cook'] for i,value in enumerate(selected)]
        inventory_cooks=[entry['cook'] for entry in scene['fallbacks']]+[scene['quadrants'][i]['cook'] for i,value in enumerate(selected) if value<4]
        pending=camera_state['pending']
        if pending and pending['pending']:
            future=scene['quadrants'][pending['region']]['cook'];inventory_cooks.append(future);staging_models=[decode(future)]
        models=[decode(cook) for cook in cooks];inventory_models=[decode(cook) for cook in inventory_cooks]
        pixel_tables=[material_pixels(cook,model[3]) for cook,model in zip(cooks,models)]
        factors=[material_factors(cook) for cook in cooks]
    timing_proof=None
    if timed:
        from world_timing_reference import verify as verify_timings
        timing_proof=verify_timings(json.loads((out/'timings.json').read_bytes()),camera_report,scene,exe,frames,
            camera_report['route_frames'],int(os.environ.get('THEIA_WORLD_SCENE_WARMUP','10')),crowd,
            json.loads(os.environ.get('THEIA_WORLD_SCENE_CAPS','[67108864,33554432,4194304]')))
    cap=rd.OpenCaptureFile();controller=None
    try:
        success(cap.OpenFile(capture,'',None));result,controller=cap.OpenCapture(rd.ReplayOptions(),None);success(result)
        actions=list(walk(controller.GetRootActions()))
        draws=[a for a in actions if a.flags&rd.ActionFlags.MeshDispatch and a.flags&rd.ActionFlags.Indirect and not a.flags&rd.ActionFlags.MultiAction]
        indirect_calls=[a for a in actions if a.flags&rd.ActionFlags.PushMarker and a.flags&rd.ActionFlags.MultiAction and a.GetName(controller.GetStructuredFile()).startswith('vkCmdDrawMeshTasksIndirectCountEXT(')]
        require(len(indirect_calls)==4+int(crowd is not None),'Expected world/proxy indirect count calls')
        present=next(a for a in actions if a.flags&rd.ActionFlags.Present)
        controller.SetFrameEvent(present.eventId,True)
        textures=controller.GetTextures();buffers=controller.GetBuffers()
        color=next(t for t in textures if t.resourceId==present.copyDestination);width,height=color.width,color.height
        rows=json.loads(subprocess.check_output([str(exe),'--benchmark-reference','0',str(frame%camera_report['route_frames']),str(camera_report['route_frames']),str(width),str(height)]))['rows'] if camera else json.loads(subprocess.check_output([str(exe),'--world-pair-reference',str(width),str(height),str(view)]))['rows']
        proxies=crowd_reference.verify(controller,buffers,scene,selected,exe,crowd,frame,camera_report['route_frames'],width,height) if crowd is not None else None
        if covered:
            # refAllResources retains inactive allocation metadata, but RenderDoc
            # 1.46 does not save unused buffer initial contents. Check the exact
            # live inventory separately; payload/raster checks cover selected sets.
            expected_buffers=Counter(len(blob) for model in inventory_models for blob in model[0])
            expected_buffers[256]+=4
            if crowd is not None:expected_buffers.update([40704,376160])
            for model in staging_models:
                expected_buffers.update(len(blob) for blob in model[0])
                expected_buffers.update(len(pixels) for pixels in model[3])
            require(Counter(b.length for b in buffers)==expected_buffers,'Protected live buffer inventory differs')
        for model in models:
            for blob in model[0]:
                require(any(bytes(controller.GetBufferData(b.resourceId,0,len(blob)))==blob for b in buffers if b.length==len(blob)),
                        'World GPU root differs: %s (%d bytes)'%(hashlib.sha256(blob).hexdigest(),len(blob)))
        expected_images=sorted(p for model in models for p in model[3])
        images=[t for t in textures if (t.width,t.height)==(64,64)]
        raw_images=Counter(bytes(controller.GetTextureData(t.resourceId,rd.Subresource())) for t in images)
        expected_count=sum(len(model[3]) for model in inventory_models)
        require(len(images)==expected_count and not (Counter(expected_images)-raw_images),
                'World selected GPU textures or retained image inventory differs')
        actual=[]
        for b in buffers:
            if b.length==256:
                work=controller.GetBufferData(b.resourceId,0,256)
                count,mask=struct.unpack_from('<2I',work,36)
                require(struct.unpack_from('<4I',work,0)==(int(count>0),1,1,int(count>0)),'Indirect command/count disagrees with compaction')
                require(count<=32,'Work compaction exceeds profile')
                ids=sorted(struct.unpack_from('<%dI'%count,work,64));actual.append((count,mask,ids))
        require(len(actual)==4,'Expected four independent work arenas')
        expected=[];visible=[]
        for _,_,hulls,_ in models:
            ids=[]
            for index,hull in enumerate(hulls):
                common=63
                for point in hull:
                    x,y,z,w=[dot(row[:3],point)+row[3] for row in rows]
                    common &= int(x < -w)|(int(x>w)<<1)|(int(y < -w)<<2)|(int(y>w)<<3)|(int(z<0)<<4)|(int(z>w)<<5)
                if common==0:ids.append(index)
            visible.append(set(ids));expected.append((len(ids),sum(1<<i for i in ids),ids))
        require(sorted(actual)==sorted(expected),'World GPU compactions differ')
        require(len(draws)==sum(int(count>0) for count,_,_ in expected)+int(crowd is not None),'Indirect subdraw count differs from world/proxy compactions')
        identity=next(t for t in textures if t.format.compType==rd.CompType.UInt and t.format.compCount==2)
        depth=next(t for t in textures if t.format.compType==rd.CompType.Depth)
        identities=controller.GetTextureData(identity.resourceId,rd.Subresource())
        depths=controller.GetTextureData(depth.resourceId,rd.Subresource());rgb=controller.GetTextureData(color.resourceId,rd.Subresource())
        limits=[{i:sum(t[0]==i for t in model[1]) for i in range(1,len(model[2])+1)} for model in models]
        populations=[0]*4;proxy_pixels=0
        for pixel in range(width*height):
            ident,primitive=struct.unpack_from('<2I',identities,pixel*8);d=struct.unpack_from('<f',depths,pixel*4)[0]
            require(math.isfinite(d) and 0<=d<=1,'Invalid world depth')
            if not ident:require(d==1,'Background depth changed')
            elif ident&0x80000000:
                local=(ident&0x7fffffff)-1
                require(proxies is not None and local in proxies['visible'] and primitive<(16 if proxies['objects'][local][3] else 12) and d<1,'Invalid proxy identity/depth/detail')
                proxy_pixels+=1
            else:
                chunk,local=ident>>29,ident&0x1fffffff
                require(chunk<4 and local>0 and local-1 in visible[chunk] and primitive<limits[chunk][local] and d<1,
                        'Invalid world identity/primitive namespace')
                populations[chunk]+=1
        require(any(populations) if camera else all(populations) if view else not any(populations),'Incomplete world coverage')
        probes={(int(width*x/12),int(height*y/10)) for x in range(1,12) for y in range(1,10)}
        for model in models:
            for triangle in model[1][::max(1,len(model[1])//32)]:
                center=[sum(p[a] for p in triangle[2])/3 for a in range(3)]
                clip=[dot(row[:3],center)+row[3] for row in rows]
                if clip[3]>0:
                    x,y=[int((clip[a]/clip[3]+1)*.5*size) for a,size in enumerate((width,height))]
                    if 0<=x<width and 0<=y<height:probes.add((x,y))
        if proxies:
            for _,center,_,_ in proxies['rays'][::max(1,len(proxies['rays'])//32)]:
                clip=[dot(row[:3],center)+row[3] for row in rows]
                if clip[3]>0:
                    x,y=[int((clip[a]/clip[3]+1)*.5*size) for a,size in enumerate((width,height))]
                    if 0<=x<width and 0<=y<height:probes.add((x,y))
        samples=[];materials=set();occlusion=[0]*4;boundaries=0;cross_occlusion=[0,0]
        for x,y in sorted(probes):
            hits=[ray(rows,model[1],x,y,width,height) for model in models]
            chunk=min(range(4),key=lambda i:hits[i][2]);ident,primitive,d,material,uv=hits[chunk]
            if sum(bool(h[0]) for h in hits)>1:occlusion[chunk]+=1
            if ident:
                ident|=chunk<<29
                coordinates=[{math.floor(v*64+delta)%64 for delta in (-.002,.002)} for v in uv]
                boundaries+=int(any(len(v)>1 for v in coordinates))
                expected_rgb=[]
                for tx in coordinates[0]:
                    for ty in coordinates[1]:
                        texel=pixel_tables[chunk][material][(ty*64+tx)*4:(ty*64+tx)*4+3]
                        expected_rgb.append([encode(linear(value)*factor) for value,factor in zip(texel,factors[chunk][material])])
            else:expected_rgb=[list(map(encode,[.025,.07,.12]))]
            if proxies:
                proxy_id,proxy_primitive,proxy_depth=crowd_reference.pixel_reference(rows,proxies['rays'],x,y,width,height)
                if ident and proxy_id:cross_occlusion[int(proxy_depth<d)]+=1
                if proxy_id and proxy_depth<d:
                    ident,primitive,d=proxy_id|0x80000000,proxy_primitive,proxy_depth
                    expected_rgb=[[encode(value) for value in crowd_reference.color_reference(proxy_id,primitive,d,3)]]
            if ident and (ident&0x80000000)==0:materials.add((chunk,material))
            pixel=y*width+x;actual=struct.unpack_from('<2I',identities,pixel*8)
            require(actual==(ident,primitive) and abs(struct.unpack_from('<f',depths,pixel*4)[0]-d)<=2e-5,
                    'World ray/raster differs at %d,%d: %s vs %s'%(x,y,actual,(ident,primitive)))
            actual_rgb=list(rgb[pixel*4:pixel*4+3])
            if color.format.BGRAOrder():actual_rgb.reverse()
            require(any(all(abs(a-b)<=2 for a,b in zip(actual_rgb,candidate)) for candidate in expected_rgb),
                    'World array/material colour differs at %d,%d chunk=%s material=%s uv=%s id=%s: %s vs %s'%(x,y,chunk,material,uv,actual,actual_rgb,expected_rgb))
            samples.append(dict(pixel=[x,y],identity=actual,color=actual_rgb))
        save=rd.TextureSave();save.resourceId=color.resourceId;save.destType=rd.FileType.PNG
        success(controller.SaveTexture(save,str(out/'preview.png')))
        report.update(passed=True,timing_proof=timing_proof,crowd=proxies['proof'] if proxies else None,proxy_pixels=proxy_pixels,cross_occlusion_probes=cross_occlusion,capture=capture,view=view,frame=frame,retirement=retirement,camera=camera_report,camera_state=camera_state,pixels=populations,images=len(images),
            protected_owners=bool(covered),selected_chunks=selected,root_buffers=len(inventory_models)*4,
            retained_inventory_verified=bool(covered),payload_scope="selected_graphs",
            compactions_verified=True,identity_depth_bounds_verified=True,sampled_materials=sorted(materials),
            ray_samples=samples,occlusion_probes=occlusion,boundary_envelope_probes=boundaries)
    finally:
        if controller is not None:controller.Shutdown()
        cap.Shutdown()


report=dict(passed=False,renderdoc=rd.GetVersionString(),validation_in_capture=False);code=0
try:qualify(report)
except Exception:report['error']=traceback.format_exc();code=1
finally:
    out=Path(os.environ['THEIA_WORLD_SCENE_OUTPUT']);out.mkdir(parents=True,exist_ok=True)
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
os._exit(code)
