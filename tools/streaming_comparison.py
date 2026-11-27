"""Independent per-frame coverage checks and explicitly defined detail-readiness observations."""
from world_camera_reference import require


def selections(values):
    require(len(values) == 4 and all(v in (r, r+4) for r, v in enumerate(values)), 'Missing/foreign region coverage')
    return {v for v in values if v < 4}


def episodes(trace, engine):
    """First selected fine within each contiguous shared near-demand window.

    Includes warmup windows. A missing observation remains null, never zero latency.
    This measures sampled view readiness, not an instrumented file/queue latency.
    """
    active = {}
    result = []
    for row in trace:
        wanted = set(row['near_demand'])
        for region in list(active):
            if region not in wanted:
                active.pop(region)['end_frame_exclusive'] = row['frame']
        for region in sorted(wanted):
            if region not in active:
                point = dict(region=region, start_frame=row['frame'], end_frame_exclusive=None,
                             first_fine_frame=None, observed_delay_frames=None, observed_delay_ms=None,
                             warmup=row['warmup'])
                point['_start_ms'] = row[engine+'_elapsed_ms']
                result.append(point)
                active[region] = point
            point = active[region]
            if region in row[engine+'_selected'] and point['first_fine_frame'] is None:
                point['first_fine_frame'] = row['frame']
                point['observed_delay_frames'] = row['frame'] - point['start_frame']
                point['observed_delay_ms'] = row[engine+'_elapsed_ms'] - point['_start_ms']
    for point in active.values():
        point['end_frame_exclusive'] = len(trace)
    for point in result:
        del point['_start_ms']
    return result


def compare(fixture, source, reference, oracle, slots):
    require(len(source['samples']) == len(reference['samples']) == len(fixture['frames']), 'Incomplete streaming pair')
    trace = []
    previous = set()
    for index, (a, b, pose) in enumerate(zip(source['samples'], reference['samples'], fixture['frames'])):
        require(a['frame'] == b['frame'] == index and a['scene_time'] == b['scene_time'] == pose['time'], 'Camera/proxy clocks differ')
        current = selections(a['world']['selected'])
        other = selections(b['selected'])
        world = a['world']
        require(len(current) <= slots and world['epoch'] == 1 + world['publications'] + world['evictions'], 'Theia slots/epoch differs')
        caps = [source['metadata']['bundle_cpu_budget'], source['metadata']['aggregate_gpu_budget']-world['fixed_gpu_bytes'], source['metadata']['image_gpu_budget']]
        oracle.state(world, world['pending'], caps)
        pending_gpu=(world['pending'] or {}).get('pending_gpu_bytes',0)
        require(world['total_gpu_bytes'] == world['physical_bytes'] + world['fixed_gpu_bytes'] + pending_gpu, 'Theia current/pending/fixed allocation charge differs')
        plan = oracle.plan(pose['eye'], previous, slots)
        added, removed = current-previous, previous-current
        require(len(added)+len(removed) <= 1 and added <= set(plan['loads']) and removed <= set(plan['removals']), 'Theia published unwanted/unrelated detail')
        previous = current
        # Independent of either backend's hysteresis/committed residency. This
        # common observation target is deliberately distinct from its own plan.
        wanted = oracle.plan(pose['eye'], set(), slots)['desired']
        trace.append(dict(frame=index, warmup=index < fixture['warmup'], near_demand=wanted,
                          theia_selected=sorted(current), bevy_selected=sorted(other),
                          theia_missing=sorted(set(wanted)-current), bevy_missing=sorted(set(wanted)-other),
                          theia_elapsed_ms=a['frame_pacing']['elapsed_ms'], bevy_elapsed_ms=b['frame_pacing']['elapsed_ms']))
    measured = trace[fixture['warmup']:]
    require(measured, 'No measured streaming observations')
    result = dict(trace=trace, measured_frames=len(measured),
                  matched_selection_frames=sum(r['theia_selected'] == r['bevy_selected'] for r in measured),
                  max_frame_start_alignment_ms=max(abs(r['theia_elapsed_ms']-r['bevy_elapsed_ms']) for r in measured),
                  near_demand_region_frames=sum(len(r['near_demand']) for r in measured),
                  scope='Shared near-demand means nearest up-to-slot-count regions within 20 m, independent of retention. Summaries exclude warmup; windows include it. View-sampled first-fine delays are not I/O/GPU/publication instrumentation. Null means never observed within that window. Retention, serial backend retirement and readiness may produce different selections.')
    for engine in ('theia', 'bevy'):
        windows = episodes(trace, engine)
        result[engine] = dict(missing_near_detail_region_frames=sum(len(r[engine+'_missing']) for r in measured),
                              windows=windows, unresolved_windows=sum(w['first_fine_frame'] is None for w in windows))
    return result


def verify_hud(report):
    require(report['overlay'], 'Streaming HUD disabled')
    for sample in report['samples']:
        rows = sample['hud_text'].splitlines()
        state = sample['stream_probe']
        camera = state['camera']
        expected = ['BEVY STATS', 'PROXIES '+str(report['players']), 'REGIONS 4',
                    'FINE %d / %d' % (len(selections(sample['selected'])), camera['slots']),
                    'PUB %d EVICT %d' % (camera['publications'], camera['evictions']),
                    'ENCODED KIB %.1f' % (state['cpu_unique_bytes']/1024),
                    'LOGICAL KIB %.1f' % (state['logical_asset_bytes']/1024),
                    'CANCEL %d DEFER %d' % (camera['cancellations'], camera['cpu_deferrals']+camera['logical_deferrals'])]
        require(len(rows) == 11 and all(row in rows for row in expected), 'Streaming HUD state/row count differs')
        if sample['frame'] > 0 and report['gpu_timestamps']:
            expected_gpu = 'GPU MS %.3f' % report['samples'][sample['frame']-1]['gpu_render_graph_ms']
            require(rows[3] == expected_gpu, 'HUD did not use previous completed GPU interval')
    return dict(frames=len(report['samples']), rows=11, scope='Exported UI text/state; no new streaming HUD pixel/glyph oracle')
