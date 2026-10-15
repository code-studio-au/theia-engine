"""Independent clocks, per-frame residency/counters and finite benchmark summaries."""
import math
import struct
from world_camera_reference import Oracle,require


def f32(value):return struct.unpack('<f',struct.pack('<f',value))[0]


def verify(report,camera,scene,renderer,frames,period,warmup,players,caps,overlay=False):
    oracle=Oracle(scene,renderer);fixed=camera['fixed_gpu_bytes']
    require(report['schema']==1 and report['workload']=='theia-streamed-mmo-v1' and report['streaming'] and
        report['overlay']==overlay and not report['authority'] and not report['skinned_animation'] and
        report['players']==players and report['frames']==frames and report['warmup_frames']==warmup and
        report['static_objects'] is None,'Wrong streamed timing scope/configuration')
    metadata=report['metadata']
    require(metadata['scene_kind']=='streamed-world' and metadata['camera_route_frames']==period and
        metadata['aggregate_gpu_budget']==caps[1] and metadata['bundle_cpu_budget']==caps[0] and
        metadata['image_gpu_budget']==caps[2] and metadata['work_requested_bytes']==377184 and
        'excludes upload' in metadata['gpu_timestamp_scope'] and 'starts at frame zero' in metadata['warmup_scope'],
        'Missing/wrong timing clock/allocation scope')
    if 'hud_fragment_storage_buffers' in metadata:require(metadata['hud_fragment_storage_buffers']==(21 if overlay else 16),'HUD descriptor profile differs')
    oracle.verify(camera,frames,[caps[0],caps[1]-fixed,caps[2]])
    samples=report['samples'];require(len(samples)==frames,'Incomplete timing samples')
    previous=None;mutations=0;max_live=0
    metrics=('frame_interval_ms','cpu_draw_ms','gpu_total_ms','gpu_compute_ms','gpu_visibility_ms','gpu_resolve_ms')
    for frame,sample in enumerate(samples):
        require(sample['frame']==frame and sample['warmup']==(frame<warmup),'Frame/warmup accounting differs')
        expected=f32(f32(60.*f32(frame%period))/f32(period-1))
        require(abs(sample['scene_time']-expected)<1e-6,'Camera clock delayed by warmup or failed to repeat')
        for key in metrics:require(math.isfinite(sample[key]) and sample[key]>=0,'Nonfinite/negative timing metric')
        require(sample['frame_interval_ms']>0 and sample['tracked_image_and_work_allocation_bytes']>=377184,'Missing interval/owned allocation metric')
        require(abs(sample['gpu_total_ms']-sum(sample[k] for k in metrics[3:]))<1e-7,'GPU stage timings do not partition frame interval')
        state=sample['world'];fine=oracle.state(state,state['pending'],[caps[0],caps[1]-fixed,caps[2]])
        if 'selected_images' in state:
            require(state['selected_images']==sum(len(scene['quadrants' if value<4 else 'fallbacks'][i]['cook']['textures']) for i,value in enumerate(state['selected'])),'Selected texture binding count differs')
        elif overlay:raise RuntimeError('HUD report is missing selected texture count')
        pending=state['pending'];pending_gpu=pending['pending_gpu_bytes'] if pending else 0
        require(state['frame']==frame and state['fixed_gpu_bytes']==fixed and state['total_gpu_bytes']==state['physical_bytes']+fixed+pending_gpu and
            state['total_gpu_bytes']<=caps[1] and state['route_cycle']==frame//period and
            state['epoch']==1+state['publications']+state['evictions'] and len(fine)<=camera['fine_slots'],'World per-frame epoch/charge differs')
        max_live=max(max_live,state['total_gpu_bytes'])
        if previous is None:require(state['epoch']==1 and not fine,'Timing route skipped coarse bootstrap')
        else:
            before={i for i in previous['selected'] if i<4};pub=state['publications']-previous['publications'];evict=state['evictions']-previous['evictions']
            require((pub,evict) in ((0,0),(1,0),(0,1)) and state['epoch']==previous['epoch']+pub+evict and state['deferrals']>=previous['deferrals'],'World mutation/counter sequence differs')
            if pub or evict:
                mutations+=1;eye=oracle.camera(frame,period)['eye'];plan=oracle.plan(eye,before,camera['fine_slots'])
                if pub:
                    added=fine-before;require(len(added)==1 and before<=fine and next(iter(added)) in plan['loads'],'Ineligible fine publication')
                    old=previous['pending'];require(old and old['pending'] and old['region']==next(iter(added)),'Fine published without prior complete pending upload')
                else:require(plan['removals'] and before-fine=={plan['removals'][0]} and fine<=before,'Wrong camera-demand eviction')
            else:require(fine==before,'Selection changed without epoch/counter mutation')
        previous=state
    require(previous['publications']==camera['publications'] and previous['evictions']==camera['evictions'] and
        previous['deferrals']==camera['cpu_deferrals']+camera['gpu_deferrals'] and
        report['stream_outcome']=={'publications':previous['publications'],'deferrals':previous['deferrals']},'Final stream counters differ')
    for key in ('epoch','selected','physical_bytes','cpu_unique_bytes','image_bytes','images','owned_chunks','protected_epochs'):
        require(previous[key]==camera['final'][key],'Timing/camera final selection differs')
    measured=[s for s in samples if not s['warmup']];require(len(measured)==frames-warmup,'Measured sample count differs')
    for key in metrics:
        values=sorted(s[key] for s in measured)
        summary={'mean':sum(values)/len(values),'max':values[-1]}
        summary.update({label:values[math.ceil((len(values)-1)*p)] for label,p in [('p50',.5),('p95',.95),('p99',.99)]})
        require(all(abs(report['summary_ms'][key][label]-value)<1e-8 for label,value in summary.items()),'Reported timing summaries differ from measured samples')
    return dict(samples=frames,measured=len(measured),mutations_checked=mutations,max_graph_pending_table_bytes=max_live,
        timing_statistics_verified=True,route_clock_includes_warmup=True)
