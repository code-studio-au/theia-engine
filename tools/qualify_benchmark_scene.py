#!/usr/bin/env python3
"""Check static-world source -> normalized -> cooked coverage and Rust admission.

Offline correctness evidence only; does not claim benchmark integration or timing.
"""
import argparse
from collections import Counter
import json
import os
import re
from pathlib import Path
import struct
import subprocess
from cook_benchmark_scene import MATERIALS, digest, f32, geometry, texture

ROOT = Path(__file__).resolve().parent.parent


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(triangle):
    return min(triangle[i:] + triangle[:i] for i in range(3))


def verify(entry, primitives):
    source = Path(entry['source']).read_bytes()
    require(digest(source) == entry['source_sha256'], 'Source digest mismatch')
    raw = Path(entry['normalized']).read_bytes()
    require(digest(raw) == entry['normalized_sha256'], 'Normalized digest mismatch')
    normalized = json.loads(raw)
    expected_positions, expected_uvs, expected_indices, expected_materials = [], [], [], []
    for material, primitive in enumerate(primitives):
        offset = len(expected_positions)
        expected_positions.extend([list(p) for p in primitive['positions']])
        expected_uvs.extend([[f32(v) for v in uv] for uv in primitive['uvs']])
        expected_indices.extend(offset + i for i in primitive['indices'])
        expected_materials.extend([material] * (len(primitive['indices']) // 3))
    for field, expected in [('positions', expected_positions), ('texcoords', expected_uvs),
                            ('indices', expected_indices), ('triangle_materials', expected_materials)]:
        require(normalized[field] == expected, 'Normalizer changed source ' + field)
    cook = entry['cook']

    def checked(record):
        data = Path(record['path']).read_bytes()
        require(digest(data) == record['content_id'], 'Content digest mismatch')
        require(len(data) == record['bytes'], 'Encoded byte count mismatch')
        return data

    mesh = checked(cook)
    meta, vertices, meshlets, refs = struct.unpack_from('<4I', mesh, 12)
    offset = 76 + ((meta + 3) & ~3)
    positions = [list(p) for p in struct.iter_unpack('<3f', mesh[offset:offset + vertices * 12])]
    require(positions == expected_positions, 'Cooked positions changed')
    clusters_offset = offset + vertices * 12
    refs_offset = clusters_offset + meshlets * 16
    tris_offset = refs_offset + refs * 4
    global_refs = struct.unpack_from(f'<{refs}I', mesh, refs_offset)
    triangles = []
    for cluster in range(meshlets):
        vo, to, vc, tc = struct.unpack_from('<4I', mesh, clusters_offset + cluster * 16)
        require(1 <= vc <= 64 and 1 <= tc <= 64, 'Meshlet output bounds')
        for t in range(tc):
            triangles.append(tuple(global_refs[vo + mesh[tris_offset + to + t * 3 + k]] for k in range(3)))
    material = checked(cook['materials'])
    meta, colors, clusters, triangle_count = struct.unpack_from('<4I', material, 12)
    offset = 96 + ((meta + 3) & ~3) + colors * 16 + (clusters + 1) * 4
    assignments = struct.unpack_from(f'<{triangle_count}I', material, offset)
    require(len(triangles) == len(assignments), 'Cooked triangle/material count')
    expected = Counter((canonical(tuple(expected_indices[i:i + 3])), expected_materials[i // 3])
                       for i in range(0, len(expected_indices), 3))
    actual = Counter((canonical(t), m) for t, m in zip(triangles, assignments))
    require(actual == expected, 'Native reordering changed winding/coverage/material')
    uv = checked(cook['uv'])
    meta, count = struct.unpack_from('<2I', uv, 12)
    offset = 88 + ((meta + 3) & ~3)
    require([list(p) for p in struct.iter_unpack('<2f', uv[offset:offset + count * 8])] == expected_uvs,
            'Cooked UV rows changed')
    require(len(cook['textures']) == 4, 'Missing texture source dependencies')
    table = checked(cook['texture_table'])
    meta, materials, images = struct.unpack_from('<3I', table, 12)
    require((materials, images) == (4, 4), 'Texture table counts')
    offset = 88 + ((meta + 3) & ~3)
    dependencies = [table[offset + i * 40:offset + i * 40 + 32].hex() for i in range(images)]
    mapping = struct.unpack_from('<4I', table, offset + images * 40)
    require([dependencies[i] for i in mapping] == [t['content_id'] for t in cook['textures']],
            'Cooked texture table changed material bindings')
    for i, record in enumerate(cook['textures']):
        image = checked(record)
        width, height = struct.unpack_from('<2I', image, 20)
        offset, size, unpacked = struct.unpack_from('<3Q', image, 80)
        require((width, height, size, unpacked) == (64, 64, 16384, 16384), 'Texture dimensions')
        require(image[offset:offset + size] == texture(i), 'Native PNG decode changed source texels')
    require(1 <= meshlets <= 32, 'Current GPU viewer meshlet cap')
    return {'vertices': vertices, 'meshlets': meshlets, 'triangles': len(triangles),
            'source_roundtrip': True, 'cooked_coverage_winding_material_uv': True,
            'texture_texels': 64 * 64 * len(MATERIALS),
            'bundle': cook['bundle']['path']}


def gpu_smoke(scene, output, gpu, dri_prime):
    from headless_display import owned_display
    result = {'world_route_verified': False, 'runtime': [], 'pressure': []}
    exe = ROOT / 'apps/theia-render-lab/target/debug/theia-render-lab'

    def execute(command, name, env):
        path = output / (name + '.log')
        with path.open('w') as log:
            subprocess.run(list(map(str, command)), stdout=log, stderr=subprocess.STDOUT,
                           check=True, timeout=120, env=env)
        text = path.read_text()
        require(all(v in text for v in ['Native surface created (Wayland)', 'Presented 120 frames',
                    'Validation totals (including teardown): 0 warnings, 0 errors']),
                'Incomplete native Wayland validation/teardown: ' + name)
        return text

    with owned_display(output / 'display', dri_prime) as (env, _, display):
        result['display'] = display
        bundles = [q['cook']['bundle']['path'] for q in scene['quadrants']]
        base = [exe, '--gpu', gpu, '--frames', '120']
        for i, bundle in enumerate(bundles):
            text = execute(base + ['--bundle', bundle], f'gpu-quadrant-{i}', env)
            image_bytes = re.search(r'4 hash-sorted images, (\d+) aggregate physical image bytes', text)
            require(image_bytes is not None, 'Missing measured image allocation')
            result['runtime'].append({'quadrant': i, 'frames': 120, 'zero_diagnostics': True,
                                      'image_bytes': int(image_bytes[1])})
            print('Headless native Wayland bundle:', i, flush=True)
        stream = base + ['--bundle', bundles[0], '--stream-bundle', bundles[3]]
        text = execute(stream, 'gpu-stream-baseline', env)
        charges = re.search(r'active (\d+) \+ pending (\d+) physical bytes', text)
        require(charges is not None, 'Missing measured active/pending allocations')
        gpu_cap = int(charges[1]) + int(charges[2])
        image_cap = result['runtime'][0]['image_bytes'] + result['runtime'][3]['image_bytes']
        unique = {}
        for index in (0, 3):
            cook = scene['quadrants'][index]['cook']
            for record in [cook, cook['materials'], cook['uv'], cook['texture_table'],
                           cook['bundle'], *cook['textures']]:
                unique[record['content_id']] = record['bytes']
        cpu_cap = sum(unique.values())
        for flag, cap in [('--bundle-gpu-budget', gpu_cap), ('--texture-budget', image_cap),
                          ('--bundle-byte-budget', cpu_cap)]:
            for short in (False, True):
                name = flag[2:] + ('-short' if short else '-exact')
                text = execute(stream + [flag, cap - int(short)], name, env)
                match = re.search(r'Bundle qualification: (\d+) complete publications, (\d+) budget deferrals', text)
                require(match is not None, 'Missing streaming outcome: ' + name)
                pubs, defers = int(match[1]), int(match[2])
                require((pubs == 0 and defers >= 2) if short else (pubs >= 2 and defers == 0),
                        'Unexpected exact/short pressure behavior: ' + name)
                result['pressure'].append({'flag': flag, 'budget': cap - int(short),
                                           'short': short, 'publications': pubs, 'deferrals': defers,
                                           'frames': 120, 'zero_diagnostics': True})
                print('Headless world pressure:', name, flush=True)
        result.update(active_plus_pending_gpu_bytes=gpu_cap, active_plus_pending_image_bytes=image_cap,
                      unique_pair_encoded_bytes=cpu_cap)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--gpu-smoke', action='store_true', help='Owned headless Wayland load and pressure checks')
    parser.add_argument('--gpu', default='7900')
    parser.add_argument('--headless-dri-prime')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'passed': False, 'runtime_integrated': False, 'quadrants': []}
    try:
        scene = json.loads(args.scene.read_bytes())
        require(scene['passed'] and scene['cooked'] and len(scene['quadrants']) == 4,
                'Incomplete cooked scene')
        parts, _ = geometry()
        env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'}
        for i, (entry, primitives) in enumerate(zip(scene['quadrants'], parts)):
            result = verify(entry, primitives)
            command = ['cargo', 'run', '--locked', '--manifest-path',
                       str(ROOT / 'crates/theia-assets/Cargo.toml'), '--example', 'bundle',
                       '--', result['bundle']]
            with (args.output / f'admit-{i}.log').open('w') as log:
                subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT,
                               timeout=120, env=env)
            result['rust_complete_bundle_admission'] = True
            report['quadrants'].append(result)
        require(sum(v['triangles'] for v in report['quadrants']) == 2048, 'Scene triangle coverage')
        if args.gpu_smoke:
            report['gpu_smoke'] = gpu_smoke(scene, args.output, args.gpu, args.headless_dri_prime)
        report.update(passed=True, source_objects=scene['total_objects'], total_triangles=2048,
                      texture_texels_checked=65536)
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError, struct.error) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
