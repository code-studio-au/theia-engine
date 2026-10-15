#!/usr/bin/env python3
"""Qualify cooked route/timings/pressure with separate headless validation and replay."""
import argparse
import json
import math
import os
from pathlib import Path
import re
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
    parser.add_argument('--qrenderdoc', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--whole-world', action='store_true', help='Qualify the complete resident world instead of quadrant replacement')
    parser.add_argument('--gpu', default='7900')
    parser.add_argument('--headless-dri-prime')
    args = parser.parse_args()
    os.chdir(ROOT)
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    args.scene = args.scene.resolve()
    report = {'passed': False, 'runtime': [], 'captures': [], 'negative': [],
              'scope': 'resident-whole-world' if args.whole_world else 'quadrant-replacement'}
    exe = ROOT / 'apps/theia-render-lab/target/debug/theia-render-lab'

    def execute(command, output, env):
        with output.open('w') as log:
            subprocess.run(list(map(str, command)), stdout=log, stderr=subprocess.STDOUT,
                           env=env, check=True, timeout=120)

    try:
        scene = json.loads(args.scene.read_bytes())
        require(scene['passed'] and scene['cooked'], 'Incomplete cooked source manifest')
        bundles = [q['cook']['bundle']['path'] for q in scene['quadrants']]
        for flags in [['--benchmark-cooked'],
                      ['--benchmark-cooked', '--bundle', bundles[0], '--benchmark-players', '1'],
                      ['--benchmark-cooked', '--bundle', bundles[0], '--frustum-test'],
                      ['--benchmark-cooked', '--bundle', bundles[0], '--benchmark-seconds', 'NaN'],
                      ['--benchmark-cooked', '--bundle', bundles[0], '--wsi-smoke-test']]:
            result = subprocess.run([str(exe), *flags], capture_output=True, text=True, timeout=20)
            require(result.returncode and 'Native surface created' not in result.stdout,
                    'Invalid cooked plan reached Vulkan')
            report['negative'].append({'flags': flags, 'error': result.stderr})
        with owned_display(args.output / 'display', args.headless_dri_prime) as (env, capture_env, display):
            report['display'] = display

            def runtime(name, flags, cooked=True, outcome=None):
                output = args.output / name
                output.mkdir(parents=True, exist_ok=True)
                path = output / 'report.json'
                path.unlink(missing_ok=True)
                command = [exe, *flags, '--gpu', args.gpu, '--frames', '240',
                           '--benchmark-warmup', '30', '--benchmark-report', path]
                execute(command, output / 'run.log', env)
                text = (output / 'run.log').read_text()
                require(all(v in text for v in ['Native surface created (Wayland)', 'Presented 240 frames',
                            'Validation totals (including teardown): 0 warnings, 0 errors']),
                        'Missing complete validated route: ' + name)
                timings = json.loads(path.read_bytes())
                workload = 'theia-cooked-bundle-route-v1' if cooked else 'theia-procedural-mmo-v1'
                require(timings['workload'] == workload and len(timings['samples']) == 240
                        and timings['samples'][-1]['scene_time'] == 60,
                        'Incomplete route or incorrect workload label')
                require(all(math.isfinite(v) and v >= 0 for metric in timings['summary_ms'].values()
                            for v in metric.values()), 'Invalid timing summary')
                require(sum(not s['warmup'] for s in timings['samples']) == 210, 'Incorrect warmup exclusion')
                if cooked:
                    require(timings['overlay'] == ('--benchmark-no-overlay' not in flags) and timings['players'] == 0
                            and timings['static_objects'] is None, 'Cooked report claims procedural content')
                if outcome is not None:
                    state = timings['stream_outcome']
                    require(timings['streaming'], 'Missing streaming workload label')
                    require((state['publications'] == 0 and state['deferrals'] >= 2) if outcome == 'defer'
                            else (state['publications'] >= 2 and state['deferrals'] == 0), 'Wrong route pressure result')
                export(timings, path)
                page = path.with_suffix('.html').read_text()
                require(('Theia cooked bundle benchmark' in page) == cooked, 'Incorrect HTML workload label')
                report['runtime'].append({'case': name, 'frames': 240, 'measured_frames': 210,
                                          'stream_outcome': timings['stream_outcome'], 'report': str(path)})
                print('Validated route:', name, flush=True)
                return text

            if args.whole_world:
                require('whole_world' in scene, 'Missing complete resident world')
                runtime('whole-world', ['--benchmark-cooked', '--bundle', scene['whole_world']['cook']['bundle']['path']])
            else:
                for i, bundle in enumerate(bundles):
                    runtime(f'quadrant-{i}', ['--benchmark-cooked', '--bundle', bundle])
                stream = ['--benchmark-cooked', '--bundle', bundles[0], '--stream-bundle', bundles[3]]
                text = runtime('stream-baseline', stream, outcome='publish')
                match = re.search(r'active (\d+) \+ pending (\d+) physical bytes', text)
                require(match is not None, 'Missing measured bundle allocation')
                gpu_cap = int(match[1]) + int(match[2])
                image_sizes = [re.search(r'4 hash-sorted images, (\d+) aggregate physical image bytes',
                              (args.output / f'quadrant-{i}' / 'run.log').read_text()) for i in (0, 3)]
                require(all(v is not None for v in image_sizes), 'Missing measured image allocations')
                image_cap = sum(int(v[1]) for v in image_sizes)
                unique = {}
                for i in (0, 3):
                    c = scene['quadrants'][i]['cook']
                    for r in [c, c['materials'], c['uv'], c['texture_table'], c['bundle'], *c['textures']]:
                        unique[r['content_id']] = r['bytes']
                for flag, cap in [('--bundle-gpu-budget', gpu_cap), ('--texture-budget', image_cap),
                                  ('--bundle-byte-budget', sum(unique.values()))]:
                    for short in (False, True):
                        name = flag[2:] + ('-short' if short else '-exact')
                        runtime(name, stream + [flag, str(cap - int(short))], outcome='defer' if short else 'publish')
                runtime('procedural-regression', ['--benchmark', '--benchmark-players', '1000'], cooked=False)
            selected = scene['whole_world']['cook']['bundle']['path'] if args.whole_world else bundles[0]
            runtime('cooked-no-overlay', ['--benchmark-cooked', '--bundle', selected, '--benchmark-no-overlay'])
            runtime('procedural-no-overlay', ['--benchmark', '--benchmark-players', '1000', '--benchmark-no-overlay'], cooked=False)
            sampled = set()
            for quadrant in ((None,) if args.whole_world else (0, 3)):
                for frame in (10, 90, 170):
                    output = args.output / f'capture-{quadrant}-{frame}'
                    output.mkdir(parents=True, exist_ok=True)
                    replay_env = capture_env.copy()
                    replay_env.update(THEIA_COOKED_CAPTURE_OUTPUT=str(output),
                                      THEIA_COOKED_SCENE=str(args.scene), THEIA_COOKED_QUADRANT=str(quadrant if quadrant is not None else 0),
                                      THEIA_COOKED_WHOLE_WORLD='1' if args.whole_world else '0',
                                      THEIA_COOKED_FRAME=str(frame), THEIA_COOKED_GPU=args.gpu)
                    execute([args.qrenderdoc, '--python', 'tools/renderdoc_cooked_benchmark.py'],
                            output / 'capture.log', replay_env)
                    capture = json.loads((output / 'report.json').read_bytes())
                    require(capture['passed'], 'Incomplete cooked replay')
                    sampled.update(capture['sampled_materials'])
                    report['captures'].append(capture)
                    print('Verified cooked replay:', quadrant, frame, flush=True)
            # Exercise the cooked panel both resident and after an actual generation publication.
            for label, streaming, cap in ([('resident', False, None)] if args.whole_world else [('resident', False, None), ('stream', True, None), ('defer', True, gpu_cap - 1)]):
                output = args.output / ('hud-' + label)
                output.mkdir(parents=True, exist_ok=True)
                replay_env = capture_env.copy()
                replay_env.update(THEIA_COOKED_CAPTURE_OUTPUT=str(output), THEIA_COOKED_SCENE=str(args.scene),
                                  THEIA_COOKED_QUADRANT='0', THEIA_COOKED_WHOLE_WORLD='1' if args.whole_world else '0',
                                  THEIA_COOKED_FRAME='70' if streaming else '90', THEIA_COOKED_GPU=args.gpu, THEIA_COOKED_OVERLAY='1')
                if streaming:
                    replay_env['THEIA_COOKED_STREAM_QUADRANT'] = '3'
                if cap is not None:
                    replay_env['THEIA_COOKED_GPU_BUDGET'] = str(cap)
                execute([args.qrenderdoc, '--python', 'tools/renderdoc_cooked_benchmark.py'], output / 'capture.log', replay_env)
                capture = json.loads((output / 'report.json').read_bytes())
                require(capture['passed'] and capture['overlay_verified'], 'Cooked stats panel was not verified')
                if cap is not None:
                    require(capture['stream_outcome']['publications'] == 0 and capture['stream_outcome']['deferrals'] >= 2, 'HUD capture did not exercise budget deferral')
                elif streaming:
                    require(capture['stream_outcome']['publications'] >= 2, 'HUD capture did not exercise publication')
                report['captures'].append(capture)
                print('Verified cooked HUD:', label, flush=True)
            output = args.output / 'procedural-hud'
            output.mkdir(parents=True, exist_ok=True)
            replay_env = capture_env.copy()
            replay_env.update(THEIA_BENCHMARK_CAPTURE_OUTPUT=str(output), THEIA_BENCHMARK_PLAYERS='1000',
                              THEIA_BENCHMARK_CAPTURE_FRAME='90', THEIA_BENCHMARK_GPU=args.gpu, THEIA_BENCHMARK_OVERLAY='1')
            execute([args.qrenderdoc, '--python', 'tools/renderdoc_benchmark.py'], output / 'capture.log', replay_env)
            report['procedural_capture'] = json.loads((output / 'report.json').read_bytes())
            require(report['procedural_capture']['passed'] and report['procedural_capture']['overlay_verified'], 'Procedural HUD regressed')
            require(sampled == set(range(4)), 'Not all four material bindings were pixel-qualified')
            report['sampled_materials'] = sorted(sampled)
        report['passed'] = True
    except (OSError, RuntimeError, ValueError, KeyError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        parser.exit(1, str(error) + '\n')
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
