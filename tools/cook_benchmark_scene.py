#!/usr/bin/env python3
"""Generate and cook a bounded static world for the future cooked-scene benchmark.

Four independently admitted quadrants reuse the existing glTF -> meshoptimizer ->
KTX -> indexed bundle pipeline. This does not switch the live benchmark renderer.
"""
import argparse
import base64
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import sys
import zlib

ROOT = Path(__file__).resolve().parent.parent
MATERIALS = ('ground', 'walls', 'roof', 'trees')
COLORS = ((82, 115, 48), (166, 140, 110), (124, 67, 49), (45, 99, 53))
CUBE = ((0, 2, 1), (1, 2, 3), (4, 5, 6), (5, 7, 6),
        (0, 1, 4), (1, 5, 4), (2, 6, 3), (3, 6, 7),
        (0, 4, 2), (2, 4, 6), (1, 3, 5), (3, 7, 5))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def height(x, y):
    return f32(math.sin(x * .035) * math.cos(y * .035) * 1.5)


def texture(index):
    """Opaque, deterministic 64x64 sRGB diagnostic checker, generated offline."""
    color = COLORS[index]
    return bytes(channel for y in range(64) for x in range(64)
                 for channel in (*[max(0, v - (20 if ((x // 8 + y // 8) & 1) else 0))
                                   for v in color], 255))


def png(rgba):
    def chunk(kind, data):
        return (struct.pack('>I', len(data)) + kind + data
                + struct.pack('>I', zlib.crc32(kind + data)))
    scanlines = b''.join(b'\0' + rgba[y * 256:(y + 1) * 256] for y in range(64))
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>2I5B', 64, 64, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(scanlines, 9)) + chunk(b'IEND', b''))


def geometry():
    # One dense primitive per material per quadrant, positions in engine Z-up.
    parts = [[{'positions': [], 'indices': [], 'uvs': []} for _ in MATERIALS]
             for _ in range(4)]
    counts = [{'ground': 0, 'buildings': 0, 'trees': 0} for _ in range(4)]

    def add(part, material, positions, triangles, uvs=None):
        primitive = parts[part][material]
        offset = len(primitive['positions'])
        positions = [tuple(map(f32, p)) for p in positions]
        primitive['positions'].extend(positions)
        primitive['indices'].extend(offset + v for t in triangles for v in t)
        primitive['uvs'].extend(uvs if uvs is not None else
                                [(p[0] / 8, p[1] / 8) for p in positions])

    def quadrant(x, y):
        return int(x >= 0) + 2 * int(y >= 0)

    for y in range(16):
        for x in range(16):
            x0, y0 = (x - 8) * 8, (y - 8) * 8
            part = quadrant(x0 + 4, y0 + 4)
            add(part, 0, [(xx, yy, height(xx, yy))
                          for yy in (y0, y0 + 8) for xx in (x0, x0 + 8)],
                [(0, 1, 2), (1, 3, 2)])
            counts[part]['ground'] += 1
    for y in range(8):
        for x in range(8):
            cx, cy = (x - 3.5) * 12, (y - 3.5) * 12
            cz = height(cx, cy) + 3
            part = quadrant(cx, cy)
            corners = [(cx + (2.5 if c & 1 else -2.5),
                        cy + (2.5 if c & 2 else -2.5),
                        cz + (3 if c & 4 else -3)) for c in range(8)]
            add(part, 1, corners, CUBE)
            roof = [(cx + dx * 2.5, cy + dy * 2.5, cz + 3)
                    for dx, dy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            add(part, 2, roof + [(cx, cy, cz + 6)],
                [(i, (i + 1) % 4, 4) for i in range(4)],
                [(0, 0), (1, 0), (1, 1), (0, 1), (.5, .5)])
            counts[part]['buildings'] += 1
    for i in range(128):
        phase = ((i * 2_654_435_761 & 0xffffffff) >> 16) * math.tau / 65536
        cx, cy = f32(math.sin(phase) * 55), f32(math.cos(phase) * 55)
        z = height(cx, cy)
        part = quadrant(cx, cy)
        points = [(cx + dx * .7, cy + dy * .7, z)
                  for dx, dy in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        add(part, 3, points + [(cx, cy, z + 6)],
            [(i, (i + 1) % 4, 4) for i in range(4)],
            [(0, 0), (1, 0), (1, 1), (0, 1), (.5, 1)])
        counts[part]['trees'] += 1
    return parts, counts


def gltf(primitives):
    data = bytearray()
    views, accessors, output = [], [], []

    def accessor(values, components, kind, fmt, target):
        offset = len(data)
        flat = [v for row in values for v in row]
        data.extend(struct.pack('<' + fmt * len(flat), *flat))
        views.append({'buffer': 0, 'byteOffset': offset,
                      'byteLength': len(data) - offset, 'target': target})
        entry = {'bufferView': len(views) - 1, 'componentType': kind,
                 'count': len(values), 'type': components}
        if components == 'VEC3':
            entry.update(min=[min(p[i] for p in values) for i in range(3)],
                         max=[max(p[i] for p in values) for i in range(3)])
        accessors.append(entry)
        return len(accessors) - 1

    for material, primitive in enumerate(primitives):
        # Inverse of the normalizer's RH Y-up -> RH Z-up conversion.
        positions = [(x, z, -y) for x, y, z in primitive['positions']]
        pos = accessor(positions, 'VEC3', 5126, 'f', 34962)
        uv = accessor(primitive['uvs'], 'VEC2', 5126, 'f', 34962)
        indices = accessor([(i,) for i in primitive['indices']], 'SCALAR', 5125, 'I', 34963)
        output.append({'attributes': {'POSITION': pos, 'TEXCOORD_0': uv},
                       'indices': indices, 'material': material, 'mode': 4})
    return {'asset': {'version': '2.0', 'generator': 'Theia bounded benchmark scene v1'},
            'extensionsUsed': ['KHR_materials_unlit'], 'scene': 0,
            'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}],
            'meshes': [{'primitives': output}], 'bufferViews': views, 'accessors': accessors,
            'buffers': [{'byteLength': len(data),
                         'uri': 'data:application/octet-stream;base64,' + base64.b64encode(data).decode()}],
            'materials': [{'name': name, 'doubleSided': True,
                           'extensions': {'KHR_materials_unlit': {}},
                           'pbrMetallicRoughness': {'baseColorFactor': [1, 1, 1, 1],
                               'baseColorTexture': {'index': i}}}
                          for i, name in enumerate(MATERIALS)],
            'images': [{'uri': name + '.png'} for name in MATERIALS],
            'textures': [{'source': i, 'sampler': 0} for i in range(4)],
            'samplers': [{'magFilter': 9728, 'minFilter': 9728, 'wrapS': 10497, 'wrapT': 10497}]}


def generate(directory):
    directory.mkdir(parents=True, exist_ok=True)
    parts, counts = geometry()
    for i, name in enumerate(MATERIALS):
        (directory / (name + '.png')).write_bytes(png(texture(i)))
    paths = []
    for i, primitives in enumerate(parts):
        path = directory / f'quadrant-{i}.gltf'
        write_json(path, gltf(primitives))
        paths.append(path)
    return paths, counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--source-only', action='store_true')
    parser.add_argument('--importer', type=Path)
    parser.add_argument('--library', type=Path)
    parser.add_argument('--meshoptimizer-source', type=Path)
    parser.add_argument('--ktx', type=Path)
    parser.add_argument('--ktx-build', type=Path)
    parser.add_argument('--ktx-source', type=Path)
    args = parser.parse_args()
    if not args.source_only and not all((args.importer, args.library, args.meshoptimizer_source,
                                         args.ktx, args.ktx_build, args.ktx_source)):
        parser.error('Cooking needs importer, library, meshoptimizer-source, ktx, ktx-build and ktx-source')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    # A failed rerun cannot retain a previous successful summary.
    report = {'schema': 1, 'passed': False, 'workload': 'theia-static-world-source-v1',
              'runtime_integrated': False, 'authority': False, 'quadrants': []}
    try:
        sources, counts = generate(output / 'sources')
        env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'}
        for i, source in enumerate(sources):
            entry = {'source': str(source), 'source_sha256': digest(source.read_bytes()),
                     'objects': counts[i]}
            if not args.source_only:
                normalized = output / f'quadrant-{i}.json'
                with (output / f'import-{i}.log').open('w') as log:
                    subprocess.run([str(args.importer.resolve()), str(source), str(normalized),
                                    '--unlit-textures'], stdout=log, stderr=subprocess.STDOUT,
                                   env=env, check=True, timeout=120)
                directory = output / f'quadrant-{i}'
                command = [sys.executable, str(ROOT / 'tools/cook_meshlet_fixture.py'),
                           '--geometry', str(normalized), '--library', str(args.library.resolve()),
                           '--source-archive', str(args.meshoptimizer_source.resolve()),
                           '--ktx', str(args.ktx.resolve()), '--ktx-build', str(args.ktx_build.resolve()),
                           '--ktx-source-archive', str(args.ktx_source.resolve()),
                           '--output-dir', str(directory)]
                with (output / f'cook-{i}.log').open('w') as log:
                    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                                   env=env, check=True, timeout=240)
                cooked = json.loads((directory / 'imported-report.json').read_bytes())
                if cooked['meshlets'] > 32:
                    raise ValueError('Quadrant exceeds current bounded GPU meshlet profile')
                entry.update(normalized=str(normalized),
                             normalized_sha256=digest(normalized.read_bytes()), cook=cooked)
            report['quadrants'].append(entry)
        if not args.source_only:
            unique = {}
            for entry in report['quadrants']:
                cook = entry['cook']
                for record in [cook, cook['materials'], cook['uv'], cook['texture_table'],
                               cook['bundle'], *cook['textures']]:
                    unique[record['content_id']] = record['bytes']
            report.update(unique_encoded_files=len(unique),
                          unique_encoded_bytes=sum(unique.values()))
        report.update(passed=True, cooked=not args.source_only,
                      generator_sha256=digest(Path(__file__).read_bytes()),
                      total_objects={name: sum(c[name] for c in counts) for name in counts[0]},
                      total_triangles=2048,
                      limits='four independent bundles; each <=32 meshlets; opaque unlit nearest mip0')
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        write_json(output / 'scene.json', report)
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
