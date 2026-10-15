#!/usr/bin/env python3
"""Compare bounded Rust ground queries with independent source-triangle heights.

CPU presentation contract only: no GPU execution, timing or gameplay collision.
"""
import argparse
import json
from pathlib import Path
import random
import struct
import subprocess


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def triangles(entry):
    normalized = json.loads(Path(entry['normalized']).read_bytes())
    points = [struct.unpack('<3f', struct.pack('<3f', *p)) for p in normalized['positions']]
    return [tuple(points[v] for v in normalized['indices'][3*i:3*i+3])
            for i, material in enumerate(normalized['triangle_materials']) if material == 0]


def height(triangles, point):
    # Brute-force double-precision source oracle, without the runtime cell index.
    x, y = point
    hits = []
    for a, b, c in triangles:
        denominator = (b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1])
        u = ((b[1]-c[1])*(x-c[0])+(c[0]-b[0])*(y-c[1]))/denominator
        v = ((c[1]-a[1])*(x-c[0])+(a[0]-c[0])*(y-c[1]))/denominator
        if min(u, v, 1-u-v) >= -1e-8:
            hits.append(u*a[2]+v*b[2]+(1-u-v)*c[2])
    require(hits and max(hits)-min(hits) < 1e-6, 'Source terrain is missing or ambiguous')
    return hits[0]


def qualify(args):
    out = args.output.resolve();out.mkdir(parents=True, exist_ok=True)
    report = dict(passed=False, gpu_execution=False, presentation_only=True)
    result = out/'report.json';result.write_text(json.dumps(report)+'\n')
    scene = json.loads(args.scene.read_bytes());exe = str(args.renderer.resolve())
    try:
        sources = [triangles(entry) for group in ('quadrants', 'fallbacks') for entry in scene[group]]
        # Exact grid vertices/seams plus probes on both sides of every 8 m line.
        points = [[x, y] for x in range(-64, 65, 4) for y in range(-64, 65, 4)]
        points += [[v+delta, w] if axis == 0 else [w, v+delta]
                   for v in range(-56, 57, 8) for delta in (-.0001, .0001)
                   for w in (-63, -31, -.0001, 0, .0001, 29, 63) for axis in (0, 1)]
        rng = random.Random(472)
        points += [[rng.uniform(-64, 64), rng.uniform(-64, 64)] for _ in range(4096)]
        # Actual 10,000-proxy trajectories at three distinct route positions.
        for frame in (0, 110, 180):
            data = json.loads(subprocess.check_output([exe, '--benchmark-reference', '10000', str(frame),
                '240', '960', '540'], text=True, timeout=30))
            proxy = [o['center'][:2] for o in data['objects'] if o['kind'] == 3]
            require(len(proxy) == 10000, 'Wrong proxy workload')
            points += proxy
        # Rust decodes f32 input; compare the same representable XY in the oracle.
        points = [struct.unpack('<2f', struct.pack('<2f', *p)) for p in points]
        queries = out/'points.json';queries.write_text(json.dumps(points))
        with (out/'reference.json').open('w') as log:
            subprocess.run([exe, '--world-ground-reference', str(args.scene.resolve()), str(queries)],
                           stdout=log, stderr=subprocess.PIPE, text=True, check=True, timeout=60)
        data = json.loads((out/'reference.json').read_bytes())
        require(data['presentation_only'] and not data['gpu_execution'] and data['points'] == len(points), 'Wrong reference scope')
        require(data['ground_triangles'] == [128]*4+[48]*4 and max(data['max_cell_candidates']) <= 8,
                'Ground triangle/cell bounds differ')
        require(len(data['heights']) == 16 and all(len(row) == len(points) for row in data['heights']), 'Incomplete mixed selections')
        largest_error = 0.;representation_delta = 0.
        for i, p in enumerate(points):
            slot = int(p[0] >= 0)+2*int(p[1] >= 0)
            require(data['regions'][i] == slot, 'Region boundary convention differs')
            fine, coarse = [height(sources[j], p) for j in (slot, slot+4)]
            representation_delta = max(representation_delta, abs(fine-coarse))
            for mask in range(16):
                expected = fine if mask & (1 << slot) else coarse
                error = abs(data['heights'][mask][i]-expected)
                largest_error = max(largest_error, error)
                require(error < 3e-6, 'Mixed terrain query differs from source triangles')
        require(representation_delta < .12, 'Previously qualified terrain error exceeded')
        failures = []
        for name, points_value in [('outside', [[64.001, 0]]), ('empty', []), ('nonfinite', [[float('nan'), 0]])]:
            bad = out/(name+'.json');bad.write_text(json.dumps(points_value))
            proc = subprocess.run([exe, '--world-ground-reference', str(args.scene.resolve()), str(bad)],
                                  capture_output=True, text=True, timeout=15)
            require(proc.returncode != 0 and 'Selected GPU:' not in proc.stdout, 'Invalid query reached Vulkan or succeeded')
            failures.append(name)
        report.update(passed=True, points=len(points), mixed_selections=16, height_comparisons=16*len(points),
                      max_absolute_error=largest_error, sampled_fine_coarse_delta=representation_delta,
                      max_cell_candidates=data['max_cell_candidates'], negative_cases=failures,
                      gpu_execution=False, benchmark_integration=False)
        print('Verified source terrain heights:', report['height_comparisons'], 'comparisons', flush=True)
    finally:
        result.write_text(json.dumps(report, indent=2)+'\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--renderer', type=Path, default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    qualify(parser.parse_args())
