#!/usr/bin/env python3
"""Qualify full-capacity texture replacement and one-byte-short retention on X11."""
import argparse
import hashlib
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

    def run(command, name, env=None):
        path = args.output / (name + '.log')
        with path.open('w') as log:
            subprocess.run(list(map(str, command)), env=env, stdout=log,
                           stderr=subprocess.STDOUT, check=True, timeout=90)
        return path.read_text(errors='replace')

    try:
        cook = json.loads(args.cook.read_bytes())
        first, last = cook['batches'][0], cook['batches'][-1]
        command = ['apps/theia-render-lab/target/debug/theia-render-lab', '--gpu', '7900',
                   '--bundle', first['bundle']['path'], '--stream-bundle', last['bundle']['path'],
                   '--frames', '120']
        measured = run(command, 'measured')
        match = re.search(r'4 root buffers, 16 images; active (\d+) \+ pending (\d+) physical bytes.*CPU unique encoded (\d+) bytes', measured)
        image = re.search(r'16 hash-sorted images, (\d+) aggregate physical image bytes', measured)
        if not match or not image:
            raise RuntimeError('Missing full-capacity measured charges')
        active, pending, cpu = map(int, match.groups())
        # Independent immutable byte union: shared images/geometry/UV count once.
        paths = [cook['geometry'], cook['uv']]
        paths += [texture['path'] for texture in cook['textures']]
        paths += [batch[role]['path'] for batch in [first, last]
                  for role in ['materials', 'table', 'bundle']]
        unique = {}
        for path in paths:
            data = Path(path).read_bytes()
            digest = hashlib.sha256(data).hexdigest()
            if Path(path).stem != digest:
                raise RuntimeError('Immutable fixture hash mismatch')
            unique[digest] = len(data)
        if cpu != sum(unique.values()):
            raise RuntimeError('CPU deduplicated byte charge differs from independent union')
        gpu, images = active + pending, int(image[1]) * 2
        settings = {'cpu': ('--bundle-byte-budget', cpu),
                    'gpu': ('--bundle-gpu-budget', gpu), 'images': ('--texture-budget', images)}
        report.update(active_gpu_bytes=active, pending_gpu_bytes=pending,
                      coexistence_gpu_bytes=gpu, coexistence_image_bytes=images,
                      unique_cpu_bytes=cpu, runs={}, captures={})
        for case in ['exact', 'cpu', 'gpu', 'images']:
            options = [value for key, (flag, cap) in settings.items()
                       for value in [flag, str(cap - (case == key))]]
            log = run(command + options, case)
            summary = re.search(r'Bundle qualification: (\d+) complete publications, (\d+) budget deferrals', log)
            if not summary or 'Presented 120 frames' not in log or 'Validation totals (including teardown): 0 warnings, 0 errors' not in log:
                raise RuntimeError(f'{case}: incomplete execution/validation')
            publications, deferrals = map(int, summary.groups())
            if (case == 'exact' and (publications < 2 or deferrals != 0)) or (case != 'exact' and (publications != 0 or deferrals < 2)):
                raise RuntimeError(f'{case}: incorrect publication/retention behavior')
            report['runs'][case] = {'publications': publications, 'deferrals': deferrals}
            # Frame 30 is after the first replacement opportunity and before the next.
            expected = last if case == 'exact' else first
            env = {k: v for k, v in os.environ.items() if not k.startswith('THEIA_CAPTURE_')}
            destination = args.output / ('capture-' + case)
            env.update(THEIA_CAPTURE_OUTPUT=str(destination), THEIA_CAPTURE_FRAME='30',
                       THEIA_CAPTURE_ASSET=cook['geometry'], THEIA_CAPTURE_UV_PAGE=cook['uv'],
                       THEIA_CAPTURE_MATERIAL_PAGE=expected['materials']['path'],
                       THEIA_CAPTURE_TEXTURE_TABLE=expected['table']['path'],
                       THEIA_CAPTURE_BUNDLE=expected['bundle']['path'],
                       THEIA_CAPTURE_INITIAL_BUNDLE=first['bundle']['path'],
                       THEIA_CAPTURE_STREAM_BUNDLE=last['bundle']['path'])
            for key, name in [('cpu', 'BUNDLE_BYTE'), ('gpu', 'BUNDLE_GPU'), ('images', 'TEXTURE')]:
                env['THEIA_CAPTURE_' + name + '_BUDGET'] = str(settings[key][1] - (case == key))
            run([args.qrenderdoc, '--python', 'tools/renderdoc_qualify.py'], 'capture-' + case, env)
            captured = json.loads((destination / 'report.json').read_bytes())
            indices = {p['texture_index'] for p in captured['camera_samples'] if p.get('texture_index') is not None}
            if captured.get('texture_count') != 16 or indices != set(expected['sampled_descriptor_indices']) or not all(captured.get(field) for field in ['replay_verified', 'bundle_dependencies_verified', 'texture_set_gpu_texels_verified', 'texture_table_sampling_verified']):
                raise RuntimeError(f'{case}: incomplete replacement/retention pixel proof')
            report['captures'][case] = {'sampled_indices': sorted(indices),
                                        'verified_pixels': len(captured['camera_samples'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
