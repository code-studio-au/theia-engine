#!/usr/bin/env python3
"""Produce four bounded bundles exercising all 16 resident texture descriptors.

Reuses qualified immutable geometry/UV bytes; invokes the existing offline cooks.
Each bundle assigns four consecutive material/descriptor indices to four triangles.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--ktx', type=Path, required=True)
    parser.add_argument('--build-dir', type=Path, required=True)
    parser.add_argument('--source-archive', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    tools = Path(__file__).resolve().parent

    def run(name, *options):
        return json.loads(subprocess.check_output(
            [sys.executable, str(tools / name), *map(str, options)], timeout=120,
            env={**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'}))

    try:
        reference = json.loads(args.reference.read_bytes())
        if reference['triangles'] != 4:
            raise ValueError('Capacity fixture requires exactly four cooked triangles')
        args.output_dir.mkdir(parents=True, exist_ok=True)
        source_dir = args.output_dir / 'sources'
        source_dir.mkdir(exist_ok=True)
        children = []
        for child in [reference, reference['uv']]:
            source = Path(child['path'])
            data = source.read_bytes()
            if hashlib.sha256(data).hexdigest() != child['content_id']:
                raise ValueError('Reference content digest mismatch')
            target = args.output_dir / source.name
            if source.resolve() != target.resolve():
                shutil.copyfile(source, target)
            children.append(target)
        textures = []
        for index in range(16):
            # Distinct opaque spatial patterns expose descriptor and UV errors.
            rgba = [[(index * 13 + corner * 37) % 256,
                     (index * 29 + corner * 53) % 256,
                     (index * 47 + corner * 71) % 256, 255]
                    for corner in range(4)]
            source = source_dir / f'texture-{index:02}.json'
            source.write_text(json.dumps({'schema': 1, 'width': 2, 'height': 2,
                                          'rgba': rgba}, sort_keys=True) + '\n')
            textures.append(run('cook_texture.py', '--ktx', args.ktx,
                                '--build-dir', args.build_dir,
                                '--source-archive', args.source_archive,
                                '--source', source, '--output-dir', args.output_dir))
        textures.sort(key=lambda texture: texture['content_id'])
        if len({texture['content_id'] for texture in textures}) != 16:
            raise ValueError('Expected 16 distinct texture dependencies')
        bundles = []
        for batch in range(4):
            indices = list(range(batch * 4, batch * 4 + 4))
            assignments = source_dir / f'materials-{batch}.json'
            assignments.write_text(json.dumps({'schema': 1,
                'geometry_sha256': reference['content_id'],
                'base_colors': [[1, 1, 1, 1]] * 16,
                'triangle_materials': indices}, sort_keys=True) + '\n')
            material = run('cook_material_page.py', '--mesh', children[0],
                           '--assignments', assignments, '--output-dir', args.output_dir)
            mapping = source_dir / f'table-{batch}.json'
            mapping.write_text(json.dumps({'schema': 1,
                'material_sha256': material['content_id'],
                'textures': [texture['path'] for texture in textures],
                'material_textures': list(range(16))}, sort_keys=True) + '\n')
            table = run('cook_texture_table.py', '--materials', material['path'],
                        '--assignments', mapping, '--output-dir', args.output_dir)
            bundle = run('package_texture_bundle.py', '--mesh', children[0],
                         '--materials', material['path'], '--uvs', children[1],
                         '--texture-table', table['path'], '--output-dir', args.output_dir)
            bundles.append({'sampled_descriptor_indices': indices,
                            'materials': material, 'table': table, 'bundle': bundle})
        report = {'geometry': str(children[0]), 'uv': str(children[1]),
                  'textures': textures, 'batches': bundles}
        (args.output_dir / 'cook.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report, indent=2))
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Capacity fixture failed: {error}\n')


if __name__ == '__main__':
    main()
