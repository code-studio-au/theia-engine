#!/usr/bin/env python3
"""Qualify complete GPU mip upload, fixed views, WSI and measured image budgets."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--texture-cook', type=Path, required=True)
    parser.add_argument('--qrenderdoc', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--shader-dir', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'passed': False, 'views': []}

    def execute(command, name, env=None, succeeds=True):
        path = args.output / (name + '.log')
        with path.open('w') as log:
            result = subprocess.run(list(map(str, command)), env=env, stdout=log,
                                    stderr=subprocess.STDOUT, timeout=90)
        if (result.returncode == 0) != succeeds:
            raise RuntimeError(name + ': unexpected exit ' + str(result.returncode))
        return path.read_text(errors='replace')

    def validated(command, name, succeeds=True):
        text = execute(command, name, succeeds=succeeds)
        if 'Validation totals (including teardown): 0 warnings, 0 errors' not in text:
            raise RuntimeError(name + ': missing zero-diagnostic teardown')
        return text

    try:
        reference = json.loads(args.reference.read_bytes())
        cooked = json.loads(args.texture_cook.read_bytes())
        count = len(cooked['levels'])
        views = list(range(count)) if count <= 3 else [0, count // 2, count - 1]
        exe = Path('apps/theia-render-lab/target/debug/theia-render-lab')
        base = [exe, '--gpu', '7900', '--asset', reference['path'], '--materials',
                reference['materials']['path'], '--uv-test', reference['uv']['path']]
        if args.shader_dir:
            base += ['--shader-dir', args.shader_dir]
        charge = None
        for level in views:
            name = 'view-' + str(level)
            destination = args.output / name
            env = {k: v for k, v in os.environ.items() if not k.startswith('THEIA_CAPTURE_')}
            env.update(THEIA_CAPTURE_OUTPUT=str(destination), THEIA_CAPTURE_ASSET=reference['path'],
                       THEIA_CAPTURE_MATERIAL_PAGE=reference['materials']['path'],
                       THEIA_CAPTURE_UV_PAGE=reference['uv']['path'],
                       THEIA_CAPTURE_TEXTURE_PAGE=cooked['path'], THEIA_CAPTURE_TEXTURE_MIP_LEVEL=str(level))
            if args.shader_dir:
                env['THEIA_CAPTURE_SHADER_DIR'] = str(args.shader_dir.resolve())
            execute([args.qrenderdoc, '--python', 'tools/renderdoc_qualify.py'], name, env)
            captured = json.loads((destination / 'report.json').read_bytes())
            if not all(captured.get(field) for field in ['replay_verified', 'texture_gpu_mips_verified',
                    'texture_sampling_verified', 'material_verified', 'uv_verified']):
                raise RuntimeError(name + ': incomplete mip/texel/pixel evidence')
            if captured['texture_mip_count'] != count or captured['texture_fixed_view_level'] != level:
                raise RuntimeError(name + ': wrong image mip count/view level')
            text = validated(base + ['--texture-mip-test', cooked['path'], str(level), '--wsi-smoke-test'], name + '-wsi')
            if 'Presented 120 frames' not in text:
                raise RuntimeError(name + ': incomplete WSI run')
            match = re.search(r'Resident texture upload: .*?, (\d+) physical image bytes', text)
            if not match:
                raise RuntimeError(name + ': missing measured image charge')
            if charge is not None and charge != int(match[1]):
                raise RuntimeError('Fixed mip view unexpectedly changed physical image charge')
            charge = int(match[1])
            report['views'].append({'level': level, 'gpu_levels_verified': count,
                                    'verified_pixels': len(captured['camera_samples']), 'wsi_frames': 120})
        active = base + ['--texture-mip-test', cooked['path'], str(views[-1])]
        exact = validated(active + ['--frames', '120', '--texture-budget', str(charge)], 'exact-budget')
        if 'Presented 120 frames' not in exact:
            raise RuntimeError('Exact budget did not present all frames')
        short = validated(active + ['--frames', '1', '--texture-budget', str(charge - 1)], 'short-budget', succeeds=False)
        if 'Texture image allocation exceeds physical byte budget' not in short or 'Presented ' in short:
            raise RuntimeError('Short image budget did not cleanly reject')
        invalid = execute(base + ['--texture-mip-test', cooked['path'], str(count)], 'invalid-level', succeeds=False)
        if 'Native surface created' in invalid or 'Fixed mip view is outside' not in invalid:
            raise RuntimeError('Invalid view was not rejected before Vulkan')
        bad = bytearray(Path(cooked['path']).read_bytes()); bad[-1] = 254
        corrupt = args.output / (hashlib.sha256(bad).hexdigest() + '.ktx2'); corrupt.write_bytes(bad)
        invalid = execute(base + ['--texture-mip-test', corrupt, '0'], 'invalid-alpha', succeeds=False)
        if 'Native surface created' in invalid or 'opaque texels' not in invalid:
            raise RuntimeError('Invalid mip alpha was not rejected before Vulkan')
        report.update(passed=True, physical_image_bytes=charge, mip_count=count,
                      exact_budget_passed=True, short_budget_rejected=True,
                      invalid_view_and_alpha_rejected_before_vulkan=True)
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
