#!/usr/bin/env python3
"""Qualify the 16-image fixture on a desktop GPU with separate replay/validation."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cook', type=Path, required=True)
    parser.add_argument('--qrenderdoc', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'passed': False}

    def execute(command, name, environment=None, succeeds=True):
        path = args.output / (name + '.log')
        with path.open('w') as stream:
            result = subprocess.run(list(map(str, command)), env=environment,
                                    stdout=stream, stderr=subprocess.STDOUT, timeout=90)
        log = path.read_text(errors='replace')
        if (result.returncode == 0) != succeeds:
            raise RuntimeError(f'{name}: unexpected exit {result.returncode}; see {path}')
        return log

    def validated(command, name, succeeds=True):
        log = execute(command, name, succeeds=succeeds)
        if 'Validation totals (including teardown): 0 warnings, 0 errors' not in log:
            raise RuntimeError(f'{name}: missing zero-diagnostic teardown')
        return log

    try:
        cook = json.loads(args.cook.read_bytes())
        exe = Path('apps/theia-render-lab/target/debug/theia-render-lab')
        base_env = os.environ.copy()
        capture_fields = ['replay_verified', 'bundle_dependencies_verified',
                          'texture_set_gpu_texels_verified', 'texture_table_sampling_verified']
        report['captures'] = []
        sampled = set()
        for index, batch in enumerate(cook['batches']):
            output = args.output / f'capture-{index}'
            # Remove inherited capture settings to keep this qualification isolated.
            env = {k: v for k, v in base_env.items() if not k.startswith('THEIA_CAPTURE_')}
            env.update(THEIA_CAPTURE_OUTPUT=str(output),
                       THEIA_CAPTURE_ASSET=cook['geometry'], THEIA_CAPTURE_UV_PAGE=cook['uv'],
                       THEIA_CAPTURE_MATERIAL_PAGE=batch['materials']['path'],
                       THEIA_CAPTURE_TEXTURE_TABLE=batch['table']['path'],
                       THEIA_CAPTURE_BUNDLE=batch['bundle']['path'])
            execute([args.qrenderdoc, '--python', 'tools/renderdoc_qualify.py'],
                    f'capture-{index}', environment=env)
            captured = json.loads((output / 'report.json').read_bytes())
            if captured.get('texture_count') != 16 or not all(captured.get(k) for k in capture_fields):
                raise RuntimeError(f'Incomplete replay evidence for batch {index}')
            indices = {v['texture_index'] for v in captured['camera_samples']
                       if v.get('texture_index') is not None}
            if indices != set(batch['sampled_descriptor_indices']):
                raise RuntimeError(f'Batch {index}: missing descriptor sampling: {indices}')
            sampled.update(indices)
            report['captures'].append({'batch': index, 'sampled_indices': sorted(indices),
                                       'verified_pixels': len(captured['camera_samples'])})
        if sampled != set(range(16)):
            raise RuntimeError('Incomplete descriptor range')
        batch = cook['batches'][-1]
        command = [exe, '--gpu', '7900', '--bundle', batch['bundle']['path']]
        log = validated(command + ['--wsi-smoke-test'], 'wsi')
        if 'Presented 120 frames' not in log or '10 swapchain generations' not in log:
            raise RuntimeError('Incomplete X11 WSI qualification')
        match = re.search(r'Resident texture table ready: 16 hash-sorted images, (\d+) aggregate physical image bytes', log)
        if match is None:
            raise RuntimeError('Missing measured image allocation charge')
        charge = int(match[1])
        validated(command + ['--frames', '120', '--texture-budget', str(charge)], 'exact-budget')
        short = validated(command + ['--frames', '1', '--texture-budget', str(charge - 1)],
                          'short-budget', succeeds=False)
        if 'budget' not in short.lower() or 'Presented ' in short:
            raise RuntimeError('Short budget did not reject before presentation')
        report.update(passed=True, sampled_indices=sorted(sampled), physical_image_bytes=charge,
                      wsi_frames=120, wsi_generations=10, validation_warnings=0, validation_errors=0,
                      exact_budget_passed=True, one_byte_short_rejected=True)
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
