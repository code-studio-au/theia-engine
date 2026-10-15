#!/usr/bin/env python3
"""Owned headless qualification of a resident cooked world plus GPU visual proxies."""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
from headless_display import owned_display
from run_scene_benchmark import export

ROOT = Path(__file__).resolve().parent.parent


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--qrenderdoc', type=Path, required=True)
    parser.add_argument('--gpu', default='7900')
    parser.add_argument('--headless-dri-prime')
    args = parser.parse_args()
    os.chdir(ROOT)
    args.scene = args.scene.resolve()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'passed': False, 'runtime': [], 'captures': [], 'negative': []}
    exe = ROOT / 'apps/theia-render-lab/target/debug/theia-render-lab'

    def execute(command, path, env):
        with path.open('w') as log:
            subprocess.run(list(map(str, command)), stdout=log, stderr=subprocess.STDOUT,
                           check=True, timeout=150, env=env)

    try:
        scene = json.loads(args.scene.read_bytes())
        require(scene['passed'] and scene['cooked'] and 'whole_world' in scene, 'Incomplete whole world source')
        bundle = scene['whole_world']['cook']['bundle']['path']
        base = ['--benchmark-mixed', '--bundle', bundle]
        invalid = [['--benchmark-mixed'], [*base, '--benchmark-players', '10001'],
                   [*base, '--stream-bundle', scene['quadrants'][0]['cook']['bundle']['path']],
                   ['--benchmark-mixed', '--benchmark'], ['--benchmark-cooked', '--benchmark-mixed'],
                   [*base, '--frames', '3', '--benchmark-warmup', '1'], [*base, '--benchmark-width', '0']]
        for flags in invalid:
            result = subprocess.run([str(exe), *flags], capture_output=True, text=True, timeout=20)
            require(result.returncode and 'Native surface created' not in result.stdout, 'Invalid mixed plan reached Vulkan')
            report['negative'].append({'flags': flags, 'error': result.stderr})
        with owned_display(args.output / 'display', args.headless_dri_prime) as (env, replay_env, display):
            report['display'] = display
            plans = [(f'mixed-{players}', [*base, '--benchmark-players', str(players)], 'theia-cooked-mmo-v1') for players in (0, 100, 500, 1000, 10000)]
            plans += [('mixed-no-overlay', [*base, '--benchmark-players', '1000', '--benchmark-no-overlay'], 'theia-cooked-mmo-v1'),
                      ('cooked-regression', ['--benchmark-cooked', '--bundle', bundle], 'theia-cooked-bundle-route-v1'),
                      ('procedural-regression', ['--benchmark', '--benchmark-players', '1000'], 'theia-procedural-mmo-v1')]
            for name, flags, workload in plans:
                output = args.output / name
                output.mkdir(parents=True, exist_ok=True)
                path = output / 'report.json'
                path.unlink(missing_ok=True)
                execute([exe, *flags, '--gpu', args.gpu, '--frames', '240', '--benchmark-warmup', '30', '--benchmark-report', path], output / 'run.log', env)
                text = (output / 'run.log').read_text()
                require(all(value in text for value in ['Native surface created (Wayland)', 'Presented 240 frames',
                            'Validation totals (including teardown): 0 warnings, 0 errors']), 'Missing native validation evidence')
                timings = json.loads(path.read_bytes())
                require(timings['workload'] == workload and len(timings['samples']) == 240
                        and timings['samples'][-1]['scene_time'] == 60
                        and sum(not s['warmup'] for s in timings['samples']) == 210, 'Wrong workload or timing accounting')
                require(all(math.isfinite(v) and v >= 0 for metric in timings['summary_ms'].values() for v in metric.values()), 'Invalid timings')
                export(timings, path)
                report['runtime'].append({'case': name, 'frames': 240, 'report': str(path)})
                print('Validated native mixed/profile route:', name, flush=True)
            occlusion = {'world_in_front': 0, 'proxy_in_front': 0}
            for players, frame, overlay in [(1000, 10, False), (1000, 90, False), (1000, 170, False),
                                             (0, 90, False), (100, 170, False), (10000, 170, False),
                                             (1000, 90, True), (0, 90, True), (10000, 170, True)]:
                output = args.output / f'capture-{players}-{frame}-hud{int(overlay)}'
                output.mkdir(parents=True, exist_ok=True)
                child = replay_env.copy()
                child.update(THEIA_MIXED_CAPTURE_OUTPUT=str(output), THEIA_MIXED_SCENE=str(args.scene),
                             THEIA_MIXED_PLAYERS=str(players), THEIA_MIXED_FRAME=str(frame),
                             THEIA_MIXED_OVERLAY=str(int(overlay)), THEIA_MIXED_GPU=args.gpu)
                execute([args.qrenderdoc, '--python', 'tools/renderdoc_mixed_benchmark.py'], output / 'capture.log', child)
                capture = json.loads((output / 'report.json').read_bytes())
                require(capture['passed'] and (capture.get('overlay_verified') if overlay else True), 'Incomplete mixed replay')
                for key in occlusion:
                    occlusion[key] += capture['occlusion_probes'][key]
                report['captures'].append(capture)
                print('Verified mixed replay:', players, frame, overlay, flush=True)
            require(all(v > 0 for v in occlusion.values()), 'Missing bidirectional world/proxy occlusion evidence')
            report['occlusion_probes'] = occlusion
            # The shared oracle modules and shader guards must preserve both original profiles.
            for name, script, fields in [
                ('cooked-reference-regression', 'tools/renderdoc_cooked_benchmark.py',
                 {'THEIA_COOKED_CAPTURE_OUTPUT': '', 'THEIA_COOKED_SCENE': str(args.scene), 'THEIA_COOKED_WHOLE_WORLD': '1', 'THEIA_COOKED_OVERLAY': '1'}),
                ('procedural-reference-regression', 'tools/renderdoc_benchmark.py',
                 {'THEIA_BENCHMARK_CAPTURE_OUTPUT': '', 'THEIA_BENCHMARK_PLAYERS': '1000', 'THEIA_BENCHMARK_CAPTURE_FRAME': '90', 'THEIA_BENCHMARK_OVERLAY': '1'})]:
                output = args.output / name
                output.mkdir(parents=True, exist_ok=True)
                child = replay_env.copy()
                for key, value in fields.items():
                    child[key] = str(output) if key.endswith('_OUTPUT') else value
                execute([args.qrenderdoc, '--python', script], output / 'capture.log', child)
                record = json.loads((output / 'report.json').read_bytes())
                require(record['passed'] and record['overlay_verified'], 'Shared reference/procedural shader regression')
                report[name] = record
        report['passed'] = True
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
