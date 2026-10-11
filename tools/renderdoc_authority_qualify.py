"""Run with RenderDoc 1.46 qrenderdoc --python; separate from Vulkan validation.
Captures the bounded server-owned actor fixture and compares GPU raster output
with readonly CPU/profile references, including separate encrypted viewers.
"""
import json
import os
from pathlib import Path
import subprocess
import shlex
import time
import traceback

children = []
child_logs = []
import renderdoc as rd


def require(result):
    if result != rd.ResultCode.Succeeded:
        raise RuntimeError(str(result))


def walk(actions):
    for action in actions:
        yield action
        yield from walk(action.children)


def qualify(report, output):
    root = Path.cwd()
    executable = root / 'apps/theia-render-lab/target/debug/theia-render-lab'
    if os.name == 'nt':
        executable = executable.with_suffix('.exe')
    alpha = float(os.environ.get('THEIA_AUTHORITY_ALPHA', '.5'))
    scripted = os.environ.get('THEIA_AUTHORITY_SCRIPT') == '1'
    network = os.environ.get('THEIA_AUTHORITY_NETWORK') == '1'
    faults = os.environ.get('THEIA_AUTHORITY_FAULTS') == '1'
    if faults and not network:
        raise RuntimeError('Packet faults require network capture')
    unpaired = os.environ.get('THEIA_AUTHORITY_UNPAIRED') == '1'
    laboratory = os.environ.get('THEIA_AUTHORITY_LABORATORY') == '1'
    closed = os.environ.get('THEIA_AUTHORITY_CLOSED') == '1'
    if scripted and network:
        raise RuntimeError('Select local script or network capture')
    reference_args = ['--authority-script-reference'] if scripted else ['--authority-reference', str(alpha)]
    reference = json.loads(subprocess.check_output([str(executable), *reference_args], text=True).splitlines()[-1])
    ready = None
    if network:
        server_log = output/'server.log'
        child_logs.append(server_log.open('w'))
        server_args = [str(root/'apps/theia-network-lab/target/debug/theia-network-lab'), '--serve', '--certificate', str(output/'localhost.der')]
        if os.name == 'nt': server_args[0] += '.exe'
        if laboratory: server_args.append('--laboratory-profile')
        children.append(subprocess.Popen(server_args, stdout=child_logs[-1], stderr=subprocess.STDOUT))
        deadline = time.monotonic()+10
        while time.monotonic()<deadline and children[-1].poll() is None:
            lines=server_log.read_text().splitlines()
            if lines:
                try: ready=json.loads(lines[0]);break
                except ValueError: pass
            time.sleep(.02)
        if ready is None: raise RuntimeError('Network listener did not publish readiness')
        network_args = ['--network-client',ready['address'],'--network-certificate',ready['certificate']]
        if faults: network_args.append('--network-faults')
        if not unpaired:
            child_logs.append((output/'companion.log').open('w'))
            children.append(subprocess.Popen([str(executable),'--gpu','7900',*network_args,'--frames','30' if closed else '240'],stdout=child_logs[-1],stderr=subprocess.STDOUT))
        y=0 if laboratory else -1000
        reference={'actors':[] if unpaired or closed else [
            {'id':10,'position_mm':[-1000,y],'health':100,'color':[.1,.8,.2],'half_extent_mm':250},
            {'id':20,'position_mm':[1000,y],'health':100,'color':[.8,.2,.1],'half_extent_mm':250}],
            'obstacle':None if laboratory or unpaired or closed else {'position_mm':[0,0],'half_extent_mm':100,'color':[.3,.3,.3]},
            'background':[.025,.07,.12],'source':'server-selected fixed initial profile; zero movement controls'}
    options = rd.CaptureOptions()
    options.apiValidation = False
    options.debugOutputMute = False
    arguments = '--gpu 7900 --authority-demo --authority-script --frames 600 --no-validation' if scripted else f'--gpu 7900 --authority-test --authority-alpha {alpha} --frames 180 --no-validation'
    if network:
        arguments_list=['--gpu','7900',*network_args,'--frames','180','--no-validation']
        arguments=subprocess.list2cmdline(arguments_list) if os.name=='nt' else ' '.join(shlex.quote(value) for value in arguments_list)
    result = rd.ExecuteAndInject(str(executable), str(root), arguments,
        [], str(output / 'authority'), options, False)
    require(result.result)
    target = rd.CreateTargetControl('', result.ident, 'Theia authority qualification', True)
    if target is None:
        raise RuntimeError('Cannot connect to capture target')
    capture = None
    try:
        target.QueueCapture(300 if scripted else 120 if closed else 60 if network else 10, 1)
        deadline = time.monotonic() + 30
        while target.Connected() and time.monotonic() < deadline:
            message = target.ReceiveMessage(None)
            if message.type == rd.TargetControlMessageType.NewCapture:
                capture = message.newCapture.path
        if target.Connected() or capture is None:
            raise RuntimeError('Missing capture or target failed to exit')
    finally:
        target.Shutdown()
    cap = rd.OpenCaptureFile()
    controller = None
    try:
        require(cap.OpenFile(capture, '', None))
        result, controller = cap.OpenCapture(rd.ReplayOptions(), None)
        require(result)
        actions = list(walk(controller.GetRootActions()))
        mesh = [a for a in actions if a.flags & rd.ActionFlags.MeshDispatch and not a.flags & rd.ActionFlags.MultiAction]
        if len(mesh) != 1 or tuple(mesh[0].dispatchDimension) != (3, 1, 1):
            raise RuntimeError('Expected one direct three-group mesh draw')
        if any(a.flags & rd.ActionFlags.Dispatch for a in actions):
            raise RuntimeError('Unexpected compute work for fixed push-only presentation')
        controller.SetFrameEvent(mesh[0].eventId, True)
        present = next(a for a in actions if a.flags & rd.ActionFlags.Present)
        color = next(t for t in controller.GetTextures() if t.resourceId == present.copyDestination)
        raw = controller.GetTextureData(color.resourceId, rd.Subresource())
        if len(raw) != color.width * color.height * 4:
            raise RuntimeError('Unexpected four-byte swapchain pixel layout')
        def srgb(value):
            return round((value * 12.92 if value <= .0031308 else 1.055 * value ** (1/2.4) - .055) * 255)
        rectangles = reference['actors'] + ([reference['obstacle']] if reference['obstacle'] is not None else [])
        counts = [0, 0, 0, 0]
        checked = 0
        # Dense sampled interior/exterior coverage. Omit a one-pixel boundary band;
        # this oracle does not reproduce hardware top-left/float rounding rules.
        guard = [8000 / color.width, 8000 / color.height]
        for y in range(0, color.height, 4):
            for x in range(0, color.width, 4):
                point = [(x+.5)/color.width*8000-4000, (y+.5)/color.height*8000-4000]
                chosen = 3
                boundary = False
                expected = reference['background']
                for i, rectangle in enumerate(rectangles):
                    d = [abs(point[axis]-rectangle['position_mm'][axis]) for axis in range(2)]
                    half = rectangle['half_extent_mm']
                    if any(abs(d[axis]-half) <= guard[axis] and d[1-axis] <= half+guard[1-axis] for axis in range(2)):
                        boundary = True
                    if all(v < half for v in d):
                        expected = rectangle['color']
                        chosen = i
                if boundary:
                    continue
                offset = (y*color.width+x)*4
                actual = list(raw[offset:offset+3])
                if color.format.BGRAOrder():
                    actual.reverse()
                if any(abs(a-srgb(b)) > 2 for a,b in zip(actual,expected)):
                    raise RuntimeError(f'Authority raster mismatch at {(x,y)}: {actual}, expected {expected}')
                counts[chosen] += 1
                checked += 1
        required = [counts[i] for i in range(len(rectangles))]+[counts[3]]
        if min(required) < 10:
            raise RuntimeError(f'Insufficient expected geometry/background coverage: {counts}')
        save = rd.TextureSave()
        save.resourceId = color.resourceId
        save.destType = rd.FileType.PNG
        require(controller.SaveTexture(save, str(output/'replay.png')))
        report.update(capture=capture, alpha=None if scripted else alpha, scripted=scripted, network=network, faults=faults, unpaired=unpaired, laboratory=laboratory, closed=closed, mesh_groups=[3,1,1], sampled_pixels=checked,
            actor_obstacle_background_counts=counts, reference=reference, replay_verified=True)
    finally:
        if controller is not None:
            controller.Shutdown()
        cap.Shutdown()

output = Path(os.environ.get('THEIA_CAPTURE_OUTPUT', 'build/qualification/m7-authority-capture')).resolve()
output.mkdir(parents=True, exist_ok=True)
report = {'renderdoc':rd.GetVersionString(), 'validation_enabled_in_capture':False}
code = 0
try:
    qualify(report, output)
except Exception:
    report['error'] = traceback.format_exc()
    code = 1
finally:
    for child in children:
        if child.poll() is None:
            child.terminate()
            try: child.wait(timeout=3)
            except subprocess.TimeoutExpired: child.kill();child.wait()
    for log in child_logs: log.close()
    (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
os._exit(code)
