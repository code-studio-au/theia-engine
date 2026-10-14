#!/usr/bin/env python3
"""Qualify automatic GPU mip/gradient probes, sampling, WSI and cleanup."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--texture-cook', type=Path, required=True)
    parser.add_argument('--qrenderdoc', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--shader-dir', type=Path, default=Path('build/shaders'))
    parser.add_argument('--linear-filter', action='store_true', help='Qualify opt-in bilinear sampling against the CPU oracle')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'passed': False, 'views': []}

    def execute(command, name, env=None, succeeds=True):
        path = args.output / (name + '.log')
        with path.open('w') as log:
            result = subprocess.run(list(map(str, command)), stdout=log,
                                    stderr=subprocess.STDOUT, env=env, timeout=90)
        if (result.returncode == 0) != succeeds:
            raise RuntimeError(name + ': unexpected exit ' + str(result.returncode))
        return path.read_text(errors='replace')

    def validated(command, name, succeeds=True):
        text = execute(command, name, succeeds=succeeds)
        if 'Validation totals (including teardown): 0 warnings, 0 errors' not in text:
            raise RuntimeError(name + ': missing zero-diagnostic teardown')
        return text

    try:
        scene = json.loads(args.reference.read_bytes())
        texture = json.loads(args.texture_cook.read_bytes())
        for view in range(3):
            destination = args.output / ('view-' + str(view))
            env = {k: v for k, v in os.environ.items() if not k.startswith('THEIA_CAPTURE_')}
            env.update(THEIA_CAPTURE_OUTPUT=str(destination), THEIA_CAPTURE_ASSET=scene['path'],
                       THEIA_CAPTURE_MATERIAL_PAGE=scene['materials']['path'],
                       THEIA_CAPTURE_UV_PAGE=scene['uv']['path'], THEIA_CAPTURE_TEXTURE_PAGE=texture['path'],
                       THEIA_CAPTURE_TEXTURE_AUTO_MIPS='1', THEIA_CAPTURE_SCENARIO=str(view),
                       THEIA_CAPTURE_SHADER_DIR=str(args.shader_dir.resolve()))
            if args.linear_filter: env['THEIA_CAPTURE_TEXTURE_LINEAR_FILTER']='1'
            execute([args.qrenderdoc, '--python', 'tools/renderdoc_qualify.py'], 'view-' + str(view), env)
            captured = json.loads((destination / 'report.json').read_bytes())
            if not all(captured.get(field) for field in ['replay_verified', 'texture_automatic_mips_verified',
                    'texture_gpu_mips_verified', 'texture_sampling_verified']):
                raise RuntimeError('Incomplete automatic mip replay')
            if args.linear_filter and not captured.get('texture_linear_filter_verified'):
                raise RuntimeError('Missing linear filtering replay proof')
            probes = captured['texture_footprint_probes']
            if len(probes) != 81:
                raise RuntimeError('Incomplete bounded GPU gradient probe set')
            report['views'].append({'view': view, 'pixels': len(captured['camera_samples']),
                'gradient_probes': sum(p['reason'] == 0 for p in probes),
                'fallback_probes': sum(p['reason'] == 2 for p in probes),
                'linear_distinguishable_pixels': captured.get('linear_distinguishable_pixels',0),
                'sampled_mips': sorted({p['mip_level'] for p in captured['camera_samples'] if p.get('mip_level') is not None})})
        exe = Path('apps/theia-render-lab/target/debug/theia-render-lab')
        base = [exe, '--gpu', '7900', '--asset', scene['path'], '--materials', scene['materials']['path'],
                '--uv-test', scene['uv']['path'], '--texture-auto-mips', texture['path']]
        if args.linear_filter:
            if not any(v['linear_distinguishable_pixels'] for v in report['views']):
                raise RuntimeError('Fixture does not distinguish bilinear from nearest sampling')
            base += ['--texture-linear-filter']
        log = validated(base + ['--shader-dir', args.shader_dir, '--wsi-smoke-test'], 'wsi')
        if 'Presented 120 frames' not in log:
            raise RuntimeError('Incomplete WSI execution')
        match = re.search(r'Resident texture upload: .*?, (\d+) physical image bytes', log)
        if not match:
            raise RuntimeError('Missing full-chain image charge')
        charge = int(match[1])
        exact = validated(base + ['--shader-dir', args.shader_dir, '--frames', '120', '--texture-budget', str(charge)], 'exact-budget')
        if 'Presented 120 frames' not in exact:
            raise RuntimeError('Exact image budget did not present all frames')
        short = validated(base + ['--shader-dir', args.shader_dir, '--frames', '1', '--texture-budget', str(charge - 1)], 'short-budget', False)
        if 'Texture image allocation exceeds physical byte budget' not in short or 'Presented ' in short:
            raise RuntimeError('Short image budget did not reject')
        missing = args.output / 'missing-shader'
        missing.mkdir(exist_ok=True)
        for path in args.shader_dir.glob('*.spv'):
            if path.name != 'asset.texture.auto.resolve.frag.spv':
                shutil.copyfile(path, missing / path.name)
        (missing / 'asset.texture.auto.resolve.frag.spv').unlink(missing_ok=True)
        failed = validated(base + ['--shader-dir', missing, '--frames', '1'], 'missing-shader', False)
        if 'GPU footprint probe:' not in failed or 'Presented ' in failed:
            raise RuntimeError('Missing shader did not exercise post-upload/probe cleanup')
        report.update(passed=True, linear_filter=args.linear_filter, physical_image_bytes=charge, exact_budget_passed=True,
                      short_budget_rejected=True, missing_shader_cleanup_passed=True)
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
