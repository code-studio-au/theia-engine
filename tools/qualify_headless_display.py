#!/usr/bin/env python3
"""Qualify an owned headless Weston/GL display with Wayland and Xwayland clients.

Never uses the desktop DISPLAY, changes power settings or installs system packages.
Wayland WSI and Xwayland ordinary frames/replay are separate evidence. The observed
Xwayland WSI-recovery timeout remains unqualified; this helper does not bypass it.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import time


def qualify(args):
    if sys.platform != 'linux':
        raise RuntimeError('Headless display qualification requires Linux')
    root, extra, output = args.weston_root.resolve(), args.extra_root.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    libs = root / 'usr/lib/x86_64-linux-gnu'
    dependency = extra / 'usr/lib/x86_64-linux-gnu/libxcb-composite.so.0'
    if not dependency.is_file():
        raise RuntimeError('Missing private libxcb-composite extraction; see DEVELOPMENT.md')
    report = {'passed': False, 'headless_backend': True, 'native_seat': False,
              'xwayland_wsi_recovery': False, 'wayland_replay': False, 'runs': [], 'captures': []}
    process = None
    env = os.environ.copy()
    for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'WAYLAND_SOCKET', 'XAUTHORITY']:
        env.pop(key, None)
    env['LD_LIBRARY_PATH'] = str(dependency.parent) + ':' + str(libs) + ':' + str(libs / 'weston')
    env['WESTON_MODULE_MAP'] = ';'.join([f'{p.name}={p}' for p in sorted(libs.rglob('*.so'))]
                                     + [f'{p.name}={p}' for p in sorted((root / 'usr/libexec').iterdir())])
    env['WESTON_DATA_DIR'] = str(root / 'usr/share/weston')
    if args.compositor_dri_prime:
        env['DRI_PRIME'] = args.compositor_dri_prime

    def run(command, name, child_env):
        with (output / (name + '.log')).open('w') as log:
            child = subprocess.Popen(list(map(str, command)), env=child_env, stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
            try:
                if child.wait(timeout=90) != 0:
                    raise RuntimeError(name + ': child failed')
            finally:
                # Each child gets an owned process group. Capture targets inherit
                # that group; cleanup also closes the owned display below.
                try:
                    os.killpg(child.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                if child.poll() is None:
                    try:
                        child.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
        return (output / (name + '.log')).read_text(errors='replace')

    try:
        with tempfile.TemporaryDirectory(prefix='theia-hwl-') as runtime, (output / 'weston-stdio.log').open('w') as log:
            env['XDG_RUNTIME_DIR'] = runtime
            (output / 'weston.log').write_text('')
            process = subprocess.Popen([str(root / 'usr/bin/weston'), '--no-config',
                '--backend=headless', '--renderer=gl', '--xwayland', '--width=1400', '--height=900',
                '--socket=theia-headless', '--idle-time=0', '--log=' + str(output / 'weston.log')],
                env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                deadline = time.monotonic() + 12
                display = None
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise RuntimeError('Owned Weston failed to start')
                    text = (output / 'weston.log').read_text()
                    match = re.search(r'xserver listening on display (:[0-9]+)', text)
                    if match and (Path(runtime) / 'theia-headless').exists():
                        display = match[1]
                        break
                    time.sleep(.05)
                if display is None:
                    raise RuntimeError('No owned headless display listener')
                base_env = {k: v for k, v in os.environ.items() if not k.startswith('THEIA_CAPTURE_')}
                for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'WAYLAND_SOCKET', 'XAUTHORITY']:
                    base_env.pop(key, None)
                base_env['XDG_RUNTIME_DIR'] = runtime
                modes = [('clear', ['--clear-only']), ('mesh', ['--mesh-only']), ('task', ['--direct-task']),
                         ('indirect', []), ('visibility', ['--visibility-test']),
                         ('authority', ['--authority-test']), ('demo', ['--authority-demo'])]
                precision = None
                if args.auto_mip_case:
                    scene = json.loads(args.auto_mip_case[0].read_bytes())
                    texture = json.loads(args.auto_mip_case[1].read_bytes())
                    extra_args = ['--asset', scene['path'], '--materials', scene['materials']['path'],
                                  '--uv-test', scene['uv']['path'], '--texture-auto-mips', texture['path']]
                    if args.linear_filter:
                        extra_args += ['--texture-linear-filter']
                    if args.tail_budget is not None:
                        extra_args += ['--texture-tail-budget', str(args.tail_budget)]
                    modes.append(('automatic-mips', extra_args))
                executable = Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
                for backend in ['wayland', 'xwayland']:
                    child_env = base_env.copy()
                    child_env['WAYLAND_DISPLAY' if backend == 'wayland' else 'DISPLAY'] = 'theia-headless' if backend == 'wayland' else display
                    for name, flags in modes:
                        options = ['--wsi-smoke-test'] if backend == 'wayland' else ['--frames', '120']
                        text = run([executable, '--gpu', '7900', *flags, *options], backend + '-' + name, child_env)
                        labels = ['Wayland'] if backend == 'wayland' else ['Xlib', 'Xcb']
                        if not any(f'Native surface created ({label})' in text for label in labels):
                            raise RuntimeError('Unexpected headless client surface backend')
                        label = next(label for label in labels if f'Native surface created ({label})' in text)
                        required = ['Presented 120 frames',
                                    'Validation totals (including teardown): 0 warnings, 0 errors']
                        if backend == 'wayland':
                            required += ['Smoke: zero-size draw deferred', 'Smoke: Wayland minimize/restore skipped']
                        if not all(item in text for item in required):
                            raise RuntimeError('Incomplete headless ' + backend + '/' + name + ' evidence')
                        if name == 'automatic-mips' and args.linear_filter:
                            match = re.search(r'Texture filter precision: (\d+) fraction bits', text)
                            if not match or not 4 <= int(match[1]) <= 16:
                                raise RuntimeError('Unsupported/missing selected-device filter precision')
                            precision = int(match[1])
                        report['runs'].append({'backend': backend, 'mode': name, 'frames': 120,
                                               'native_surface':label,
                                               'wsi_recovery': backend == 'wayland'})
                        print('Qualified headless:', backend, name, flush=True)
                if args.qrenderdoc:
                    capture_env = base_env.copy()
                    capture_env['DISPLAY'] = display
                    capture_env['THEIA_CAPTURE_VISIBILITY'] = '1'
                    for scenario in range(3):
                        destination = output / ('visibility-capture-' + str(scenario))
                        capture_env.update(THEIA_CAPTURE_OUTPUT=str(destination), THEIA_CAPTURE_SCENARIO=str(scenario))
                        run([args.qrenderdoc, '--python', 'tools/renderdoc_qualify.py'], 'visibility-capture-' + str(scenario), capture_env)
                        captured = json.loads((destination / 'report.json').read_bytes())
                        if not captured.get('replay_verified') or not captured.get('surface_verified'):
                            raise RuntimeError('Missing headless Xwayland visibility pixel replay')
                        report['captures'].append({'kind': 'visibility', 'scenario': scenario,
                                                  'pixels':len(captured['surface_samples'])})
                    capture_env.pop('THEIA_CAPTURE_VISIBILITY')
                    if args.auto_mip_case:
                        capture_env.update(THEIA_CAPTURE_ASSET=scene['path'], THEIA_CAPTURE_MATERIAL_PAGE=scene['materials']['path'],
                            THEIA_CAPTURE_UV_PAGE=scene['uv']['path'], THEIA_CAPTURE_TEXTURE_PAGE=texture['path'], THEIA_CAPTURE_TEXTURE_AUTO_MIPS='1')
                        if args.linear_filter:
                            capture_env.update(THEIA_CAPTURE_TEXTURE_LINEAR_FILTER='1', THEIA_CAPTURE_FILTER_PRECISION_BITS=str(precision))
                        if args.tail_budget is not None:
                            capture_env['THEIA_CAPTURE_TEXTURE_TAIL_BUDGET'] = str(args.tail_budget)
                        for scenario in range(3):
                            destination = output / ('texture-capture-' + str(scenario))
                            capture_env.update(THEIA_CAPTURE_OUTPUT=str(destination), THEIA_CAPTURE_SCENARIO=str(scenario))
                            run([args.qrenderdoc, '--python', 'tools/renderdoc_qualify.py'], 'texture-capture-' + str(scenario), capture_env)
                            captured = json.loads((destination / 'report.json').read_bytes())
                            fields = ['replay_verified', 'texture_automatic_mips_verified', 'texture_gpu_mips_verified']
                            if args.tail_budget is not None:
                                fields.append('texture_mip_tail_verified')
                            if args.linear_filter:
                                fields.append('texture_linear_filter_verified')
                            if not all(captured.get(field) for field in fields):
                                raise RuntimeError('Missing headless Xwayland texture replay')
                            report['captures'].append({'kind': 'automatic-mips', 'scenario': scenario,
                                'pixels': len(captured['camera_samples']), 'probes': len(captured['texture_footprint_probes'])})
                text = (output / 'weston.log').read_text()
                match = re.search(r'GL renderer: (.+)', text)
                report.update(passed=True, compositor_gl_renderer=match[1] if match else 'unknown',
                    private_display=display, filter_precision_bits=precision,
                    xwayland_version=subprocess.check_output(['/usr/bin/Xwayland','-version'],stderr=subprocess.STDOUT,text=True).splitlines()[0],
                    xwayland_sha256=hashlib.sha256(Path('/usr/bin/Xwayland').read_bytes()).hexdigest(),
                    dependency_sha256=hashlib.sha256(dependency.read_bytes()).hexdigest())
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        report['error'] = str(error)
        raise
    finally:
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--weston-root', type=Path, default=Path('build/qualification/wayland-tool/root'))
    parser.add_argument('--extra-root', type=Path, default=Path('build/qualification/headless-xwayland-tool/root'))
    parser.add_argument('--output', type=Path, default=Path('build/qualification/headless-display'))
    parser.add_argument('--compositor-dri-prime', help='Optional child-only Mesa GPU selector; no global change')
    parser.add_argument('--qrenderdoc', type=Path)
    parser.add_argument('--auto-mip-case', nargs=2, type=Path, metavar=('SCENE', 'TEXTURE'))
    parser.add_argument('--linear-filter', action='store_true')
    parser.add_argument('--tail-budget', type=int)
    args = parser.parse_args()
    if (args.linear_filter or args.tail_budget is not None) and not args.auto_mip_case:
        parser.error('Filter/tail options require an automatic mip fixture')
    if args.tail_budget is not None and args.tail_budget < 4:
        parser.error('Tail budget must retain at least four dense bytes')
    try:
        qualify(args)
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        parser.exit(1, str(error) + '\n')
