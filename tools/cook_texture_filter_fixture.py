#!/usr/bin/env python3
"""Cook explicitly authored alternating texels for bilinear/mip qualification.

Each level is independently authored, not downsampled from another level.
The existing pinned KTX producer performs native encoding and exact-byte validation.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['ktx', 'build-dir', 'source-archive', 'output-dir']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--profile', choices=['square', 'odd', 'thin', 'vertical'], default='square')
    args = parser.parse_args()
    try:
        width, height = {'square': (8, 8), 'odd': (5, 3), 'thin': (1024, 1), 'vertical': (1, 1024)}[args.profile]
        levels = []
        for mip in range(max(width, height).bit_length()):
            w, h = max(1, width >> mip), max(1, height >> mip)
            rgba = [[255 if (x + y) % 2 else 0, (mip * 19) % 256,
                     255 if y % 2 else (mip * 23) % 256, 255]
                    for y in range(h) for x in range(w)]
            levels.append({'width': w, 'height': h, 'rgba': rgba})
        args.output_dir.mkdir(parents=True, exist_ok=True)
        source = args.output_dir / 'source.json'
        source.write_text(json.dumps({'schema': 1, 'levels': levels}) + '\n')
        command = [sys.executable, str(Path(__file__).with_name('cook_texture_mips.py')),
                   '--source', str(source), '--output-dir', str(args.output_dir / 'assets')]
        for name in ['ktx', 'build_dir', 'source_archive']:
            command += ['--' + name.replace('_', '-'), str(getattr(args, name))]
        report = json.loads(subprocess.check_output(command, timeout=120))
        (args.output_dir / 'cook.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report, indent=2))
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Filter fixture cooking failed: {error}\n')


if __name__ == '__main__':
    main()
