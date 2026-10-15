#!/usr/bin/env python3
"""Offline qualification of distance demand + complete CPU world publication.

Uses actual benchmark camera output and checked cooked bundles. No GPU, window,
network, background I/O or runtime streamer is exercised by this tool.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def qualify(args):
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    report_path = output / 'report.json'
    report_path.write_text(json.dumps({'passed': False}) + '\n')
    scene = json.loads(args.scene.read_bytes())
    entries = scene['quadrants']
    require(scene['passed'] and len(entries) == 4, 'needs checked four-quadrant scene')
    bounds, resources, bundles = [], [], []
    for entry in entries:
        cook = entry['cook']
        bundles.append(cook['bundle']['path'])
        mesh = Path(cook['path']).read_bytes()
        meta, vertices = struct.unpack_from('<2I', mesh, 12)
        offset = 76 + ((meta + 3) & ~3)
        positions = list(struct.iter_unpack('<3f', mesh[offset:offset + vertices * 12]))
        bounds.append(([min(p[a] for p in positions) for a in (0, 1)],
                       [max(p[a] for p in positions) for a in (0, 1)]))
        paths = [cook['bundle'], cook, cook['materials'], cook['uv'],
                 cook['texture_table'], *cook['textures']]
        owned = {}
        for record in paths:
            blob = Path(record['path']).read_bytes()
            digest = hashlib.sha256(blob).hexdigest()
            require(digest == record['content_id'], 'fixture identity changed')
            owned[digest] = len(blob)
        resources.append(owned)
    union = {k: v for owned in resources for k, v in owned.items()}
    budget = sum(union.values())
    points = []
    for frame in range(240):
        raw = subprocess.run([str(args.renderer.resolve()), '--benchmark-reference',
                              '0', str(frame), '240', '1280', '720'],
                             capture_output=True, text=True, check=True, timeout=10)
        points.append(json.loads(raw.stdout)['eye'][:2])
    # Deliberate spatial excursions exercise arrival/exit and exact corner cases,
    # independent of how much the visually useful camera route approaches an edge.
    points += [[float(x), 0.] for x in range(-110, 111)]
    points += [[float(x), 0.] for x in range(110, -111, -1)]
    points += [[-100., -100.], [0., 0.], [100., 100.], [1000., 1000.]]
    route = output / 'route.txt'
    # Round-trip through f32 exactly as the Rust route reader does.
    points = [[struct.unpack('<f', struct.pack('<f', v))[0] for v in p] for p in points]
    route.write_text(''.join(' '.join(format(v, '.9g') for v in p) + '\n' for p in points))
    cases = []
    for load, retain, slots in [(20, 35, 4), (20, 35, 2), (20, 20, 2), (0, 0, 1)]:
        name = f'load{load}-retain{retain}-slots{slots}'
        command = [str(args.example.resolve()), '--world-route', str(load), str(retain),
                   str(slots), str(budget), str(route), *bundles]
        result = subprocess.run(command, capture_output=True, text=True, check=True, timeout=30)
        (output / (name + '.jsonl')).write_text(result.stdout)
        rows = [json.loads(line) for line in result.stdout.splitlines()]
        require(len(rows) == len(points), 'route sample loss')
        resident, epoch, changes, deferred, retained = set(), 0, 0, 0, 0
        for frame, (eye, row) in enumerate(zip(points, rows)):
            eligible = []
            for chunk, (lo, hi) in enumerate(bounds):
                # Independent closest-point clamp, compared with interval-gap Rust.
                closest = [max(lo[a], min(hi[a], eye[a])) for a in (0, 1)]
                squared = sum((eye[a] - closest[a]) ** 2 for a in (0, 1))
                radius = retain if chunk in resident else load
                if squared <= radius ** 2:
                    eligible.append((squared, chunk))
                    retained += int(chunk in resident and squared > load ** 2)
            eligible.sort()
            desired = [chunk for _, chunk in eligible[:slots]]
            loads = [chunk for chunk in desired if chunk not in resident]
            removals = sorted(resident - set(desired))
            omitted = [chunk for _, chunk in eligible[slots:]]
            if loads or removals:
                epoch += 1
                changes += 1
            current_union = {k: v for chunk in desired for k, v in resources[chunk].items()}
            expected = dict(frame=frame, desired=desired, loads=loads, removals=removals,
                            deferred=omitted, resident=sorted(desired), epoch=epoch,
                            live_bytes=sum(current_union.values()))
            require(row == expected, f'{name} sample {frame}: CPU demand/publication differs')
            resident = set(desired)
            deferred += len(omitted)
        require(not resident, 'final far-away sample must remove all chunks')
        cases.append(dict(case=name, samples=len(rows), transactions=changes,
                          deferred_candidates=deferred, retained_candidates=retained))
        print('Verified CPU world demand:', cases[-1], flush=True)
    require(cases[0]['retained_candidates'] > 0, 'retention band not exercised')
    require(cases[1]['deferred_candidates'] > 0, 'slot pressure not exercised')
    report = dict(passed=True, scope='offline-cpu-world-demand-v1', camera_samples=240,
                  samples_per_case=len(points), unique_fixture_bytes=budget, cases=cases,
                  scene_sha256=hashlib.sha256(args.scene.read_bytes()).hexdigest(),
                  gpu_execution=False, runtime_streaming=False)
    report_path.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--renderer', type=Path, default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--example', type=Path, default=Path('crates/theia-assets/target/debug/examples/bundle_residency'))
    qualify(parser.parse_args())
