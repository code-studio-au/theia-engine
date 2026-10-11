#!/usr/bin/env python3
"""Cook a native wide quad for visible unsupported-gradient mip-zero fallback."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from cook_uv_page import cook as cook_uv
from cook_material_page import cook as cook_material


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--library', type=Path, required=True)
    parser.add_argument('--source-archive', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    try:
        report = json.loads(subprocess.check_output([sys.executable,
            str(Path(__file__).with_name('cook_meshlet_fixture.py')), '--library', str(args.library),
            '--source-archive', str(args.source_archive), '--fixture', 'footprint-wide',
            '--output-dir', str(args.output_dir)], timeout=120))
        mesh = Path(report['path']).read_bytes()
        if report['vertices'] != 4 or report['triangles'] != 2:
            raise ValueError('Wide footprint fixture must be a two-triangle quad')
        material = {'schema': 1, 'geometry_sha256': report['content_id'],
                    'base_colors': [[1, 1, 1, 1]], 'triangle_materials': [0, 0]}
        uvs = {'schema': 1, 'geometry_sha256': report['content_id'],
               'texcoords': [[0, 0], [1, 0], [1, 1], [0, 1]]}
        for role, source, cooker, suffix in [('materials', material, cook_material, '.theia-material'),
                                             ('uv', uvs, cook_uv, '.theia-uv')]:
            encoded = json.dumps(source, sort_keys=True).encode()
            (args.output_dir / (role + '-source.json')).write_bytes(encoded)
            blob = cooker(mesh, encoded); digest = hashlib.sha256(blob).hexdigest()
            path = args.output_dir / (digest + suffix); path.write_bytes(blob)
            report[role] = {'path': str(path), 'content_id': digest, 'bytes': len(blob)}
        (args.output_dir / 'cook.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report, indent=2))
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Footprint fixture failed: {error}\n')


if __name__ == '__main__':
    main()
