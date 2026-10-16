#!/usr/bin/env python3
"""Qualify shared native benchmark deadlines, overload reporting and camera-stream compatibility."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from benchmark_clock import verify as verify_clock
from headless_display import owned_display
from qualify_bevy_camera_stream import check
from qualify_bevy_workload import verify as verify_poses
from run_bevy_comparison import run
from verify_bevy_snapshot import verify as verify_pixels
from world_camera_reference import Oracle, require

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT/'build/qualification/bevy-clock/native')
    parser.add_argument('--headless-dri-prime')
    parser.add_argument('--gpu', default='7900')
    args = parser.parse_args()
    os.chdir(ROOT)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    result = out/'report.json'
    summary = dict(passed=False, cases=[], negative=[], scope='Clock correctness and compatibility; not paired streaming performance')
    result.write_text(json.dumps(summary)+'\n')
    theia = ROOT/'apps/theia-render-lab/target/release/theia-render-lab'
    bevy = ROOT/'tools/theia-bevy-benchmark/target/release/theia-bevy-benchmark'
    command = [sys.executable, 'tools/run_bevy_comparison.py', '--scene', str(args.scene.resolve()),
               '--output', str(out/'replay'), '--players', '0', '10000', '--frames', '180', '--warmup', '10',
               '--route-frames', '120', '--pre-roll', '60', '--target-hz', '60', '--present-mode', 'immediate',
               '--gpu-timings', '--stats-overlay', '--snapshot-frames', '0', '179', '--validation', '--headless', '--skip-build', '--gpu', args.gpu]
    if args.headless_dri_prime:
        command += ['--headless-dri-prime', args.headless_dri_prime]
    run(command, os.environ.copy(), out/'replay.log', 240)
    paired = json.loads((out/'replay/comparison.json').read_bytes())
    summary['replay'] = paired
    fixture_path = out/'replay/01-10000-proxies/fixture.json'
    fixture = json.loads(fixture_path.read_bytes())
    logical = [sum(len(p['positions'])*32+len(p['texture']['rgba'])+32 for p in c['parts']) for c in fixture['chunks']]
    oracle = Oracle(json.loads(args.scene.read_bytes()), theia)
    with owned_display(out/'display', args.headless_dri_prime) as (native, _, display):
        env = native.copy()
        env.update(WGPU_ADAPTER_NAME=args.gpu, WGPU_VALIDATION='1', WGPU_DEBUG='1', VK_INSTANCE_LAYERS='VK_LAYER_KHRONOS_validation')
        for name, hz, mode, flags in [('camera', 60, 'immediate', ['--camera-stream', '--stream-capture-frames', '0,75,179']),
                                      ('overload', 240, 'fifo', [])]:
            case = out/name
            case.mkdir(exist_ok=True)
            run([str(bevy), '--fixture', str(fixture_path), '--output', str(case), '--pre-roll', '60',
                 '--target-hz', str(hz), '--present-mode', mode, *flags], env, case/'run.log')
            report = json.loads((case/'report.json').read_bytes())
            require(report['metadata']['backend'] == 'Vulkan' and report['metadata']['device_id'] == 0x744c
                    and report['metadata']['present_mode'] == mode and report['extent'] == fixture['extent'], 'Device/backend/mode/extent differs')
            require(report['source_report_sha256'] == fixture['source_report_sha256'], 'Source provenance differs')
            clock = verify_clock(report, hz)
            proof = dict(name=name, clock=clock, report=str(case/'report.json'))
            if name == 'overload':
                require(clock['frames_at_least_one_period_late'] > 0, 'Overload did not exercise deadline misses')
            else:
                proof['streaming'] = check(report, fixture, oracle, logical)
                observed = copy.deepcopy(fixture)
                for row, sample in zip(observed['frames'], report['samples']):
                    row['selected'] = sample['selected']
                observed_path = case/'observed-fixture.json'
                observed_path.write_text(json.dumps(observed, separators=(',', ':'))+'\n')
                proof['poses'] = verify_poses(observed_path, bevy, theia, case/'pose-proof.json')
                proof['pixels'] = [verify_pixels(observed_path, case/label, bevy, exclude_ambiguous_proxy_depth=True)
                                   for label in report['streaming_probe']['captures']]
            summary['cases'].append(proof)
        # Exercise the Vulkan recorder's late path as well as Bevy's main schedule.
        case = out/'theia-overload'
        run([sys.executable, 'tools/run_scene_benchmark.py', '--streamed-world', '--cooked-scene', str(args.scene.resolve()),
             '--players', '0', '--frames', '140', '--warmup', '10', '--route-frames', '120', '--target-hz', '240',
             '--present-mode', 'fifo', '--no-overlay', '--validation', '--skip-build', '--gpu', args.gpu, '--output', str(case)],
            native, out/'theia-overload.log')
        report = json.loads((case/'00-streamed-0-proxies/report.json').read_bytes())
        clock = verify_clock(report, 240)
        require(clock['frames_at_least_one_period_late'] > 0, 'Vulkan overload did not exercise misses')
        summary['cases'].append(dict(name='theia-overload', clock=clock))
        summary['display'] = display
    for executable, flags in [(theia, ['--benchmark-target-hz', '60']),
                              (theia, ['--benchmark', '--benchmark-target-hz', '0']),
                              (theia, ['--benchmark', '--benchmark-target-hz', '241']),
                              (bevy, ['--target-hz', '0']), (bevy, ['--target-hz', '241']),
                              (bevy, ['--target-hz', '60', '--snapshot', '0']),
                              (bevy, ['--target-hz', '60', '--streaming-probe', '2'])]:
        prefix = ['--fixture', str(fixture_path), '--output', str(out/'negative')] if executable == bevy else []
        process = subprocess.run([str(executable), *prefix, *flags], capture_output=True, text=True, timeout=30)
        require(process.returncode != 0, 'Invalid clock configuration accepted')
        summary['negative'].append(dict(executable=executable.name, flags=flags, returncode=process.returncode))
    for key in ('scheduled_ms', 'lateness_ms'):
        forged = copy.deepcopy(report)
        forged['samples'][10]['frame_pacing'][key] += 1
        try:
            verify_clock(forged, 240)
        except (AssertionError, RuntimeError):
            summary['negative'].append(dict(forged_clock_field=key))
        else:
            raise RuntimeError('Forged clock trace accepted')
    summary.update(passed=True, provenance={str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in (theia, bevy)})
    result.write_text(json.dumps(summary, indent=2)+'\n')
    print('Qualified:', result, flush=True)


if __name__ == '__main__':
    main()
