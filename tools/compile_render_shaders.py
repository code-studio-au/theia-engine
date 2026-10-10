#!/usr/bin/env python3
"""Compile and validate the M0-D Slang shaders; publish only after all pass."""
import argparse
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--slangc', default='slangc', help='Slang compiler executable')
    parser.add_argument('--spirv-val', default='spirv-val', help='SPIR-V validator executable')
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'build' / 'shaders')
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    shaders = [('triangle.slang', 'meshMain', 'mesh', 'triangle.mesh.spv'),
               ('triangle.slang', 'fragmentMain', 'fragment', 'triangle.frag.spv'),
               ('amplified.slang', 'taskMain', 'amplification', 'triangle.task.spv'),
               ('amplified.slang', 'amplifiedMeshMain', 'mesh', 'triangle.amplified.mesh.spv')]
    try:
        subprocess.run([args.slangc, '-version'], check=True)
        with tempfile.TemporaryDirectory(dir=args.output_dir) as temporary:
            for source, entry, stage, filename in shaders:
                output = Path(temporary) / filename
                command = [args.slangc, str(ROOT / 'shaders/render' / source),
                           '-entry', entry, '-stage', stage, '-target', 'spirv',
                           '-profile', 'spirv_1_6', '-o', str(output)]
                if stage in ('mesh', 'amplification'):
                    command += ['-capability', 'spvMeshShadingEXT']
                subprocess.run(command, check=True)
                subprocess.run([args.spirv_val, '--target-env', 'vulkan1.3', str(output)], check=True)
                print(f'Compiled and validated {stage}: {filename}', flush=True)
            for _, _, _, filename in shaders:
                (Path(temporary) / filename).replace(args.output_dir / filename)
        print(f'Shaders ready in {args.output_dir}')
    except (OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'Shader qualification failed: {error}\n')


if __name__ == '__main__':
    main()
