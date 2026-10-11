#!/usr/bin/env python3
"""Qualify native Wayland Vulkan surfaces in an owned Weston nested on X11.
Requires extracted Weston 13 packages and a desktop GPU; never changes the user's
session/compositor or system installation. Nested evidence is not native-seat evidence.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


def qualify(weston_root, output, network=False, capture=False, pixels=False, capacity_cook=None):
    if os.name != 'posix' or not os.environ.get('DISPLAY'):
        raise RuntimeError('Nested qualification requires Linux/X11 DISPLAY')
    weston_root, output = weston_root.resolve(), output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    libs = weston_root/'usr/lib/x86_64-linux-gnu'
    weston_env = os.environ.copy()
    weston_env.pop('WAYLAND_DISPLAY', None)
    weston_env.pop('WAYLAND_SOCKET', None)
    weston_env['LD_LIBRARY_PATH'] = str(libs)+':'+str(libs/'weston')
    mappings = [f'{path.name}={path}' for path in sorted(libs.rglob('*.so'))]
    mappings += [f'{path.name}={path}' for path in sorted((weston_root/'usr/libexec').iterdir())]
    weston_env['WESTON_MODULE_MAP'] = ';'.join(mappings)
    weston_env['WESTON_DATA_DIR'] = str(weston_root/'usr/share/weston')
    compositor = None
    with tempfile.TemporaryDirectory(prefix='theia-wl-') as runtime:
        weston_env['XDG_RUNTIME_DIR'] = runtime
        with (output/'weston-stdio.log').open('w') as log:
            try:
                (output/'weston.log').write_text('')
                compositor = subprocess.Popen([str(weston_root/'usr/bin/weston'),
                    '--no-config','--backend=x11','--renderer=gl','--width=1400',
                    '--height=900','--socket=theia-qualification','--idle-time=0',
                    '--log='+str(output/'weston.log'),*(['--debug'] if pixels else [])], env=weston_env, stdout=log, stderr=subprocess.STDOUT)
                deadline = time.monotonic()+10
                while not (Path(runtime)/'theia-qualification').exists():
                    if time.monotonic()>=deadline or compositor.poll() is not None:
                        raise RuntimeError('Weston failed; see '+str(output))
                    time.sleep(.02)
                client_env = os.environ.copy()
                client_env.pop('DISPLAY', None)
                client_env.pop('WAYLAND_SOCKET', None)
                client_env['WAYLAND_DISPLAY'] = 'theia-qualification'
                client_env['XDG_RUNTIME_DIR'] = runtime
                executable = Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
                for name, mode in [('clear',['--clear-only']),('mesh',['--mesh-only']),
                        ('task',['--direct-task']),('indirect',[]),
                        ('authority',['--authority-test']),('demo',['--authority-demo'])]:
                    path = output/(name+'.log')
                    with path.open('w') as client_log:
                        subprocess.run([str(executable),'--gpu','7900','--wsi-smoke-test',*mode],
                            env=client_env,stdout=client_log,stderr=subprocess.STDOUT,check=True,timeout=30)
                    text = path.read_text()
                    for required in ['Native surface created (Wayland)', 'Presented 120 frames',
                                     'Validation totals (including teardown): 0 warnings, 0 errors',
                                     'Smoke: zero-size draw deferred',
                                     'Smoke: Wayland minimize/restore skipped']:
                        if required not in text:
                            raise RuntimeError('Missing '+required+': '+text)
                    print('Qualified nested Wayland:',name,flush=True)
                capacity_report = None
                if capacity_cook:
                    fixture = json.loads(capacity_cook.read_bytes())

                    def capacity_run(name, options):
                        path = output / (name + '.log')
                        with path.open('w') as client_log:
                            subprocess.run([str(executable), '--gpu', '7900', '--wsi-smoke-test', *options],
                                env=client_env, stdout=client_log, stderr=subprocess.STDOUT,
                                check=True, timeout=30)
                        text = path.read_text()
                        for required in ['Native surface created (Wayland)', 'Presented 120 frames',
                                'Validation totals (including teardown): 0 warnings, 0 errors',
                                'Resident texture table ready: 16 hash-sorted images']:
                            if required not in text:
                                raise RuntimeError(name + ': missing ' + required)
                        generations = re.search(r'Render Lab teardown: (\d+) swapchain generations', text)
                        summary = re.search(r'Bundle qualification: (\d+) complete publications', text)
                        expected = 7 + (int(summary[1]) if summary else 0)
                        if not generations or int(generations[1]) != expected:
                            raise RuntimeError(name + ': incorrect WSI/publication generation count')
                        print('Qualified nested Wayland:', name, flush=True)
                        return text

                    for index, batch in enumerate(fixture['batches']):
                        capacity_run('capacity-' + str(index), ['--bundle', batch['bundle']['path']])
                    stream = ['--bundle', fixture['batches'][0]['bundle']['path'],
                              '--stream-bundle', fixture['batches'][-1]['bundle']['path']]
                    text = capacity_run('capacity-stream', stream)
                    charge = re.search(r'active (\d+) \+ pending (\d+) physical bytes.*CPU unique encoded (\d+) bytes', text)
                    images = re.search(r'16 hash-sorted images, (\d+) aggregate physical image bytes', text)
                    if not charge or not images:
                        raise RuntimeError('Missing Wayland capacity coexistence charges')
                    active, pending, cpu = map(int, charge.groups())
                    budgets = {'gpu': ('--bundle-gpu-budget', active + pending),
                               'cpu': ('--bundle-byte-budget', cpu),
                               'images': ('--texture-budget', int(images[1]) * 2)}
                    capacity_report = {'resident_runs': 4, 'stream_runs': {},
                                       'coexistence_budgets': {k: v[1] for k, v in budgets.items()}}
                    for case in ['exact', 'gpu', 'cpu', 'images']:
                        options = [value for key, (flag, cap) in budgets.items()
                                   for value in [flag, str(cap - (case == key))]]
                        text = capacity_run('capacity-stream-' + case, stream + options)
                        summary = re.search(r'Bundle qualification: (\d+) complete publications, (\d+) budget deferrals', text)
                        if not summary:
                            raise RuntimeError('Missing capacity replacement summary')
                        publications, deferrals = map(int, summary.groups())
                        if (case == 'exact' and (publications < 2 or deferrals)) or (case != 'exact' and (publications or deferrals < 2)):
                            raise RuntimeError('Incorrect Wayland capacity replacement: ' + case)
                        capacity_report['stream_runs'][case] = {'publications': publications, 'deferrals': deferrals}
                if pixels:
                    from qualify_wayland_pixels import qualify as qualify_pixels
                    qualify_pixels(weston_root,client_env,output)
                if network:
                    with (output/'network-harness.log').open('w') as client_log:
                        subprocess.run(['python3','tools/qualify_network_viewer.py','--faults',
                            '--repair','--wsi','--output',str(output/'network')],env=client_env,
                            stdout=client_log,stderr=subprocess.STDOUT,check=True,timeout=45)
                    for path in sorted((output/'network').glob('client-*.log')):
                        if 'Native surface created (Wayland)' not in path.read_text():
                            raise RuntimeError('Network viewer silently used another backend')
                    print('Qualified nested Wayland: encrypted full/delta/repair pair',flush=True)
                if capture:
                    capture_env = client_env.copy()
                    # Qt's debugger UI uses the host X11 session; its injected Rust
                    # target selects the owned Wayland socket, confirmed in the log.
                    capture_env['DISPLAY'] = os.environ['DISPLAY']
                    capture_env['QT_QPA_PLATFORM'] = 'xcb'
                    capture_env['XDG_DATA_HOME'] = str(Path('build/qualification/renderdoc-data').resolve())
                    capture_env['THEIA_CAPTURE_OUTPUT'] = str(output/'indirect-capture')
                    debugger=Path('build/qualification/renderdoc-tool/renderdoc_1.46/bin/qrenderdoc').resolve()
                    with (output/'capture.log').open('w') as capture_log:
                        result = subprocess.run([str(debugger),'--python','tools/renderdoc_qualify.py'],env=capture_env,
                            stdout=capture_log,stderr=subprocess.STDOUT,timeout=30)
                    if result.returncode!=0:
                        text=(output/'capture.log').read_text()
                        if 'does not support requested instance extension: VK_KHR_wayland_surface' in text:
                            raise RuntimeError('This RenderDoc build rejects VK_KHR_wayland_surface; standalone checks passed, capture remains unqualified')
                        raise RuntimeError('Capture failed; see '+str(output/'capture.log'))
                    report=json.loads((output/'indirect-capture/report.json').read_text())
                    if not report.get('replay_verified'):
                        raise RuntimeError('Indirect Wayland replay failed')
                    if 'Native surface created (Wayland)' not in (output/'capture.log').read_text():
                        raise RuntimeError('Injected target did not confirm its Wayland surface')
                    print('Qualified nested Wayland: indirect capture/replay',flush=True)
                (output/'report.json').write_text(json.dumps({'nested_compositor':'Weston 13.0.0 X11/GL',
                    'native_surface':'Wayland','standalone_modes':6,'frames_per_mode':120,
                    'network_pair':network,'compositor_pixels':pixels,'capture_replay':capture,'programmatic_minimize_restore':False,
                    'texture_capacity':capacity_report,
                    'validation':'zero warnings/errors in checked standalone runs'},indent=2)+'\n')
            finally:
                if compositor is not None and compositor.poll() is None:
                    compositor.terminate()
                    try:
                        compositor.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        compositor.kill()
                        compositor.wait()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--weston-root', type=Path, default=Path('build/qualification/wayland-tool/root'))
    parser.add_argument('--output', type=Path, default=Path('build/qualification/wayland-wsi'))
    parser.add_argument('--network',action='store_true')
    parser.add_argument('--capture',action='store_true')
    parser.add_argument('--pixels',action='store_true')
    parser.add_argument('--capacity-cook', type=Path, help='Optional 16-image fixture cook report')
    args = parser.parse_args()
    qualify(args.weston_root,args.output,args.network,args.capture,args.pixels,args.capacity_cook)
