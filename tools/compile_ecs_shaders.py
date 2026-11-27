#!/usr/bin/env python3
"""Compile and validate bounded M1 ECS reference compute shaders."""
import argparse
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--slangc', default='slangc')
    parser.add_argument('--spirv-val', default='spirv-val')
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'build/shaders')
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    stages = ['initialize', 'mutate', 'integrate', 'readRetained']
    try:
        subprocess.run([args.slangc, '-version'], check=True)
        with tempfile.TemporaryDirectory(dir=args.output_dir) as temporary:
            for entry in stages:
                output = Path(temporary) / f'ecs.{entry}.spv'
                subprocess.run([args.slangc, str(ROOT / 'shaders/ecs/reference.slang'),
                                '-entry', entry, '-stage', 'compute', '-target', 'spirv',
                                '-profile', 'spirv_1_6', '-o', str(output)], check=True)
                subprocess.run([args.spirv_val, '--target-env', 'vulkan1.3', str(output)], check=True)
                print(f'Compiled and validated ECS {entry}', flush=True)
            for entry in stages:
                (Path(temporary) / f'ecs.{entry}.spv').replace(args.output_dir / f'ecs.{entry}.spv')
    except (OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'ECS shader qualification failed: {error}\n')


if __name__ == '__main__':
    main()
