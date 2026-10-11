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


def qualify(output):
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
            process = subprocess.Popen([str(executable), '--gpu', '7900', '--authority-demo', '--frames', '600'],
                stdout=log, stderr=subprocess.STDOUT)
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
            for _ in range(4):
                key(b'space', True)
                time.sleep(.02)
                key(b'space', False)
                time.sleep(.25)
            if process.wait(timeout=20) != 0:
                raise RuntimeError('Demo failed; see '+str(output))
        text = output.read_text()
        if '4 hits' not in text or 'target health 0' not in text or '0 warnings, 0 errors' not in text:
            raise RuntimeError('Native key events did not produce four authoritative hits/death: '+text)
        print('XTest X11 D/Space press/release events reached the Rust input/authority path; four hits/death and zero Vulkan diagnostics. Log:', output)
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
        close(display)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('build/qualification/m7-interactive-native-input.log'))
    qualify(parser.parse_args().output)
