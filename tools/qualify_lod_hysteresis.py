#!/usr/bin/env python3
"""Qualify GPU-owned resident heightfield LOD history against replay state/pixels."""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fine', type=Path, required=True)
    parser.add_argument('--coarse', type=Path, required=True)
    parser.add_argument('--qrenderdoc', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--shader-dir', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'passed': False, 'trajectory': [], 'validation': []}

    def execute(command, name, env=None):
        with (args.output / (name + '.log')).open('w') as log:
            subprocess.run(list(map(str, command)), stdout=log, stderr=subprocess.STDOUT,
                           env=env, timeout=90, check=True)

    try:
        fine, coarse = str(args.fine.resolve()), str(args.coarse.resolve())
        # Fixed qualified terrain has a padded 5.03462-pixel bound at 960x540/view 2.
        budgets = [6., 7., 6., 4., 7., 6., 6., 4.]
        levels = [0, 1, 1, 0, 1, 1, 0, 0]
        reasons = [4, 0, 0, 2, 0, 0, 4, 2]
        for phase, (budget, selected, reason) in enumerate(zip(budgets, levels, reasons)):
            name = 'phase-' + str(phase)
            destination = args.output / name
            env = {k: v for k, v in os.environ.items() if not k.startswith('THEIA_CAPTURE_')}
            env.update(THEIA_CAPTURE_LOD_FINE=fine, THEIA_CAPTURE_LOD_COARSE=coarse,
                       THEIA_CAPTURE_ASSET=[fine, coarse][selected],
                       THEIA_CAPTURE_LOD_HYSTERESIS_TEST='1',
                       THEIA_CAPTURE_FRAME=str(phase * 20 + 5),
                       THEIA_CAPTURE_OUTPUT=str(destination))
            if args.shader_dir:
                env['THEIA_CAPTURE_SHADER_DIR'] = str(args.shader_dir.resolve())
            execute([args.qrenderdoc, '--python', 'tools/renderdoc_qualify.py'], name, env)
            captured = json.loads((destination / 'report.json').read_bytes())
            if not all(captured.get(key) for key in ['replay_verified', 'lod_gpu_verified',
                        'lod_both_pages_gpu_verified', 'camera_verified']):
                raise RuntimeError(name + ': incomplete work/page/pixel evidence')
            if (captured['lod_selected'], captured['lod_reason'], captured['lod_hard_budget']) != (selected, reason, budget):
                raise RuntimeError(name + ': trajectory mismatch')
            if captured['lod_reset'] != int(phase == 6):
                raise RuntimeError(name + ': incorrect history reset')
            report['trajectory'].append({key: captured[key] for key in
                ['lod_selected', 'lod_previous', 'lod_reason', 'lod_hard_budget',
                 'lod_entry_budget', 'lod_reset', 'lod_gpu_bound_pixels']})
        exe = Path('apps/theia-render-lab/target/debug/theia-render-lab')
        base = [exe, '--gpu', '7900', '--lod-test', fine, coarse]
        if args.shader_dir:
            base += ['--shader-dir', args.shader_dir]
        for name, options in [('trajectory', ['--lod-hysteresis-test', '--frames', '180']),
                ('wsi', ['--lod-hysteresis', '0.2', '--wsi-smoke-test']),
                ('disabled', ['--wsi-smoke-test'])]:
            execute(base + options, name)
            text = (args.output / (name + '.log')).read_text()
            frames = 180 if name == 'trajectory' else 120
            if f'Presented {frames} frames' not in text or 'Validation totals (including teardown): 0 warnings, 0 errors' not in text:
                raise RuntimeError(name + ': missing execution/zero-diagnostic teardown')
            report['validation'].append({'case': name, 'frames': frames, 'warnings': 0, 'errors': 0})
        report['passed'] = True
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
