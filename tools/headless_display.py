"""Owned Linux headless Weston/Xwayland environments for GPU qualifiers."""
from contextlib import contextmanager
import hashlib
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import time


@contextmanager
def owned_display(output, dri_prime=None,
                  weston_root=Path('build/qualification/wayland-tool/root'),
                  extra_root=Path('build/qualification/headless-xwayland-tool/root')):
    if sys.platform != 'linux':
        raise RuntimeError('Owned headless display requires Linux')
    output, root, extra = output.resolve(), weston_root.resolve(), extra_root.resolve()
    output.mkdir(parents=True, exist_ok=True)
    libs = root / 'usr/lib/x86_64-linux-gnu'
    dependency = extra / 'usr/lib/x86_64-linux-gnu/libxcb-composite.so.0'
    if not dependency.is_file():
        raise RuntimeError('Missing private headless display dependencies; see DEVELOPMENT.md')
    env = os.environ.copy()
    for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'WAYLAND_SOCKET', 'XAUTHORITY']:
        env.pop(key, None)
    env.update(LD_LIBRARY_PATH=str(dependency.parent) + ':' + str(libs) + ':' + str(libs / 'weston'),
               WESTON_DATA_DIR=str(root / 'usr/share/weston'))
    env['WESTON_MODULE_MAP'] = ';'.join([f'{p.name}={p}' for p in sorted(libs.rglob('*.so'))]
                                     + [f'{p.name}={p}' for p in sorted((root / 'usr/libexec').iterdir())])
    if dri_prime:
        env['DRI_PRIME'] = dri_prime
    with tempfile.TemporaryDirectory(prefix='theia-hwl-') as runtime, (output / 'stdio.log').open('w') as log:
        env['XDG_RUNTIME_DIR'] = runtime
        (output / 'weston.log').write_text('')
        process = subprocess.Popen([str(root / 'usr/bin/weston'), '--no-config', '--backend=headless',
            '--renderer=gl', '--xwayland', '--width=1400', '--height=900', '--socket=theia-headless',
            '--idle-time=0', '--log=' + str(output / 'weston.log')], env=env,
            stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            deadline = time.monotonic() + 12
            display = None
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError('Owned headless compositor failed to start')
                text = (output / 'weston.log').read_text()
                match = re.search(r'xserver listening on display (:[0-9]+)', text)
                if match and (Path(runtime) / 'theia-headless').exists():
                    display = match[1]
                    break
                time.sleep(.05)
            if display is None:
                raise RuntimeError('Owned headless compositor startup deadline expired')
            clients = {k: v for k, v in os.environ.items() if not k.startswith('THEIA_CAPTURE_')}
            for key in ['DISPLAY', 'WAYLAND_DISPLAY', 'WAYLAND_SOCKET', 'XAUTHORITY']:
                clients.pop(key, None)
            clients['XDG_RUNTIME_DIR'] = runtime
            wayland, xwayland = clients.copy(), clients.copy()
            wayland['WAYLAND_DISPLAY'] = 'theia-headless'
            xwayland['DISPLAY'] = display
            info = {'private_display': display, 'backend': 'headless/GL', 'native_seat': False,
                    'runtime_validation': 'Wayland', 'capture_replay': 'Xwayland',
                    'xwayland_wsi_recovery': False,
                    'dependency_sha256': hashlib.sha256(dependency.read_bytes()).hexdigest()}
            yield wayland, xwayland, info
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
