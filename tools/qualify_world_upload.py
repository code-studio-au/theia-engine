#!/usr/bin/env python3
"""Native Wayland qualification of isolated GPU chunk ownership and regressions."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display


def require(value, message):
    if not value:
        raise RuntimeError(message)


def qualify(args):
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    result = out / 'report.json'
    result.write_text('{"passed":false}\n')
    scene = json.loads(args.scene.read_bytes())
    bundles = [q['cook']['bundle']['path'] for q in scene['quadrants']]
    whole = scene['whole_world']['cook']['bundle']['path']
    invalid = out / 'invalid-scene.json'
    invalid.write_text('{"passed":false,"quadrants":[]}\n')
    reports = []
    plans = [
        ('owners', ['--world-upload-test', str(args.scene.resolve())], True),
        ('invalid-scene', ['--world-upload-test', str(invalid)], False),
        ('missing-scene', ['--world-upload-test', str(out / 'missing-scene.json')], False),
        ('conflicting-mode', ['--world-upload-test', str(args.scene.resolve()), '--benchmark'], False),
        ('cooked-stream-regression', ['--benchmark-cooked', '--bundle', bundles[0],
           '--stream-bundle', bundles[1], '--frames', '120', '--benchmark-warmup', '10',
           '--benchmark-report', str(out / 'cooked.json')], True),
        ('mixed-regression', ['--benchmark-mixed', '--bundle', whole,
           '--benchmark-players', '10000', '--frames', '120', '--benchmark-warmup', '10',
           '--benchmark-report', str(out / 'mixed.json')], True),
    ]
    with owned_display(out / 'display', args.headless_dri_prime) as (runtime, _, display):
        for name, command, succeeds in plans:
            log_path = out / (name + '.log')
            with log_path.open('w') as log:
                process = subprocess.run([str(args.renderer.resolve()), *command, '--gpu', args.gpu],
                                         env=runtime, stdout=log, stderr=subprocess.STDOUT, timeout=60)
            text = log_path.read_text()
            require((process.returncode == 0) == succeeds, f'{name} unexpected exit')
            if name == 'conflicting-mode':
                require('Selected GPU:' not in text and 'Vulkan' not in text,
                        'conflicting mode reached Vulkan')
            else:
                require('Validation totals (including teardown): 0 warnings, 0 errors' in text,
                        f'{name} missing zero-diagnostic teardown')
            row = dict(case=name, expected_success=succeeds, exit=process.returncode)
            if name == 'owners':
                prefix = 'World upload ownership verified: '
                proof = [line for line in text.splitlines() if line.startswith(prefix)]
                require(len(proof) == 1 and 'Presented 3 frames' in text, 'missing GPU owner proof')
                row['proof'] = json.loads(proof[0][len(prefix):])
                p = row['proof']
                require(p['chunks'] == 4 and p['root_buffers'] == 16 and p['images'] == 16,
                        'incomplete upload sets')
                require(all(p[k] for k in ('retained_reader_rejection', 'last_owner_reuse',
                    'short_aggregate_rejection', 'short_image_rejection', 'cancelled_ready_released')),
                    'missing ownership/budget evidence')
                require(not p['rendered_chunks'] and not p['asynchronous_streaming'], 'scope changed')
            if 'regression' in name:
                path = out / ('cooked.json' if name.startswith('cooked') else 'mixed.json')
                data = json.loads(path.read_bytes())
                require(data['frames'] == 120 and sum(not s['warmup'] for s in data['samples']) == 110,
                        'incomplete regression route')
                if name.startswith('cooked'):
                    require(data['samples'][-1]['cooked']['publications'] > 0, 'stream never published')
                else:
                    require(data['players'] == 10000 and data['workload'] == 'theia-cooked-mmo-v1',
                            'wrong mixed workload')
            reports.append(row)
            print('Verified native world upload case:', name, flush=True)
    result.write_text(json.dumps(dict(passed=True, display=display, cases=reports,
                                     rendered_independent_chunks=False), indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--renderer', type=Path, default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--headless-dri-prime', default='pci-0000_03_00_0')
    parser.add_argument('--gpu', default='7900')
    qualify(parser.parse_args())
