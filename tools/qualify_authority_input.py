#!/usr/bin/env python3
"""Offline Linux/X11 keyboard-event qualification for our own spawned demo only.
Temporarily focuses only our spawned demo; each key-down verifies that focus.
Restores prior focus when still owned; native key state is released during cleanup.
API references: https://www.x.org/releases/X11R7.6/doc/libX11/specs/libX11/libX11.html
https://www.x.org/releases/X11R7.5/doc/Xext/xtestlib.html
Runtime/gameplay remains Rust; this is an offline Python/native-Xlib test harness.
"""
import argparse
import ctypes as c
import ctypes.util
import os
from pathlib import Path
import subprocess
import time


def qualify(output, network=False, faults=False):
    if os.name != 'posix' or not os.environ.get('DISPLAY'):
        raise RuntimeError('This qualification requires a Linux X11 DISPLAY')
    x = c.CDLL(ctypes.util.find_library('X11'))
    def bind(name, result, args):
        fn = getattr(x, name)
        fn.restype, fn.argtypes = result, args
        return fn
    pointer, number = c.c_void_p, c.c_ulong
    open_display = bind('XOpenDisplay', pointer, [c.c_char_p])
    close = bind('XCloseDisplay', c.c_int, [pointer])
    root_window = bind('XDefaultRootWindow', number, [pointer])
    atom = bind('XInternAtom', number, [pointer, c.c_char_p, c.c_int])
    prop = bind('XGetWindowProperty', c.c_int, [pointer, number, number, c.c_long, c.c_long,
        c.c_int, number, c.POINTER(number), c.POINTER(c.c_int), c.POINTER(number),
        c.POINTER(number), c.POINTER(c.POINTER(c.c_ubyte))])
    free = bind('XFree', c.c_int, [pointer])
    keysym = bind('XStringToKeysym', number, [c.c_char_p])
    keycode = bind('XKeysymToKeycode', c.c_ubyte, [pointer, number])
    flush = bind('XFlush', c.c_int, [pointer])
    display = open_display(None)
    if not display:
        raise RuntimeError('Cannot connect to X11')
    xtst = c.CDLL(ctypes.util.find_library('Xtst'))
    fake = xtst.XTestFakeKeyEvent
    fake.restype, fake.argtypes = c.c_int, [pointer, c.c_uint, c.c_int, number]
    focus = bind('XSetInputFocus', c.c_int, [pointer, number, c.c_int, number])
    get_focus = bind('XGetInputFocus', c.c_int, [pointer, c.POINTER(number), c.POINTER(c.c_int)])
    sync = bind('XSync', c.c_int, [pointer, c.c_int])
    previous_focus, previous_revert = number(), c.c_int()
    get_focus(display, c.byref(previous_focus), c.byref(previous_revert))
    held = set()
    process = None
    companion = None
    server = None
    extra_logs = []
    window = None
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        root = root_window(display)
        def property32(window, name):
            kind, form, count, remaining = number(), c.c_int(), number(), number()
            data = c.POINTER(c.c_ubyte)()
            result = prop(display, window, atom(display, name, False), 0, 4096, False, 0,
                c.byref(kind), c.byref(form), c.byref(count), c.byref(remaining), c.byref(data))
            try:
                if result or form.value != 32 or remaining.value or not data:
                    return []
                values = c.cast(data, c.POINTER(number))
                return [values[i] for i in range(count.value)]
            finally:
                if data:
                    free(data)
        executable = Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
        with output.open('w') as log:
            arguments = [str(executable), '--gpu', '7900', '--authority-demo', '--frames', '600']
            if network:
                server_log = output.with_name(output.stem+'-server.log')
                extra_logs.append(server_log.open('w'))
                server = subprocess.Popen(['apps/theia-network-lab/target/debug/theia-network-lab','--serve'],stdout=extra_logs[-1],stderr=subprocess.STDOUT)
                deadline = time.monotonic()+10
                ready = None
                while time.monotonic()<deadline and server.poll() is None:
                    lines = server_log.read_text().splitlines()
                    if lines:
                        try:
                            ready = __import__('json').loads(lines[0])
                            break
                        except ValueError:
                            pass
                    time.sleep(.02)
                if ready is None:
                    raise RuntimeError('Listener did not publish readiness')
                arguments = [str(executable),'--gpu','7900','--network-client',ready['address'],'--network-certificate',ready['certificate'],'--frames','600']
            if faults:
                arguments.append('--network-faults')
            process = subprocess.Popen(arguments,stdout=log,stderr=subprocess.STDOUT)
            deadline = time.monotonic() + 10
            window = None
            while time.monotonic() < deadline and process.poll() is None:
                matches = [w for w in property32(root, b'_NET_CLIENT_LIST')
                    if property32(w, b'_NET_WM_PID') == [process.pid]]
                if len(matches) == 1:
                    window = matches[0]
                    break
                time.sleep(.02)
            if window is None:
                raise RuntimeError('Cannot identify exactly one window belonging to our demo PID')
            if network:
                companion_log = output.with_name(output.stem+'-companion.log')
                extra_logs.append(companion_log.open('w'))
                companion_args = [str(executable),'--gpu','7900','--network-client',ready['address'],'--network-certificate',ready['certificate'],'--frames','660']
                if faults:
                    companion_args.append('--network-faults')
                companion = subprocess.Popen(companion_args,stdout=extra_logs[-1],stderr=subprocess.STDOUT)
                deadline = time.monotonic()+5
                while 'Verified server assigned actor 10' not in output.read_text() and time.monotonic()<deadline:
                    time.sleep(.02)
                if 'Verified server assigned actor 10' not in output.read_text():
                    raise RuntimeError('Input target did not receive expected server actor assignment')
            focus(display, window, 2, 0)
            sync(display, False)
            time.sleep(.05)
            def key(name, pressed):
                code = keycode(display, keysym(name))
                actual_focus, revert = number(), c.c_int()
                get_focus(display, c.byref(actual_focus), c.byref(revert))
                if pressed and actual_focus.value != window:
                    raise RuntimeError('Our demo lost focus; refusing to send key-down elsewhere')
                if not code or not fake(display, code, pressed, 0):
                    raise RuntimeError('Cannot emit XTest key event')
                if pressed:
                    held.add(code)
                else:
                    held.discard(code)
                flush(display)
            key(b'd', True)
            time.sleep(.3)
            key(b'd', False)
            for request in range(4):
                key(b'space', True)
                time.sleep(.02)
                key(b'space', False)
                if network:
                    # Fault delays can reorder server tick arrival relative to key timing.
                    # Start cooldown spacing only after this request is acknowledged.
                    deadline = time.monotonic()+3
                    while f'Server outcome request {request} ' not in output.read_text():
                        if time.monotonic()>=deadline or process.poll() is not None:
                            raise RuntimeError('Network outcome was not acknowledged: '+output.read_text())
                        time.sleep(.02)
                time.sleep(.25)
            if process.wait(timeout=20) != 0:
                raise RuntimeError('Demo failed; see '+str(output))
        text = output.read_text()
        gameplay_ok = ('4 hits' in text and 'target health 0' in text)
        if network:
            if companion.wait(timeout=20)!=0 or server.wait(timeout=10)!=0:
                raise RuntimeError('Companion viewer or independent listener failed')
            server_text = server_log.read_text()
            gameplay_ok = '4 acknowledged outcomes' in text and 'hits: 4' in server_text and 'health: [100, 0]' in server_text
            if faults:
                from qualify_network_viewer import verify_faults
                verify_faults(text)
                verify_faults(companion_log.read_text())
            if '0 warnings, 0 errors' not in companion_log.read_text():
                raise RuntimeError('Companion Vulkan diagnostics')
        if not gameplay_ok or '0 warnings, 0 errors' not in text:
            raise RuntimeError('Native key events did not produce four authoritative hits/death: '+text)
        print('XTest D/Space qualified', 'separate encrypted authority and two GPU viewers' if network else 'local Rust authority', '; four hits/death and zero Vulkan diagnostics. Log:', output)
    finally:
        for code in held:
            fake(display, code, False, 0)
        flush(display)
        current_focus, revert = number(), c.c_int()
        get_focus(display, c.byref(current_focus), c.byref(revert))
        if window is not None and current_focus.value == window:
            focus(display, previous_focus.value, previous_revert.value, 0)
            flush(display)
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        for other in [companion, server]:
            if other is not None and other.poll() is None:
                other.terminate()
                try:
                    other.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    other.kill()
                    other.wait()
        for handle in extra_logs:
            handle.close()
        close(display)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('build/qualification/m7-interactive-native-input.log'))
    parser.add_argument('--network',action='store_true',help='spawn separate encrypted authority plus a second GPU viewer')
    parser.add_argument('--faults',action='store_true',help='post-bootstrap encrypted packet faults; requires --network')
    args=parser.parse_args()
    if args.faults and not args.network:
        parser.error('--faults requires --network')
    qualify(args.output,args.network,args.faults)
