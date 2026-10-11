"""Run with RenderDoc 1.46 qrenderdoc --python; separate from Vulkan validation.
Captures the bounded server-owned actor fixture and compares GPU raster output
with the readonly CPU authority/presentation oracle. No live-network claim.
"""
import json
import os
from pathlib import Path
import subprocess
import time
import traceback
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
    reference = json.loads(subprocess.check_output([str(executable), '--authority-reference', str(alpha)], text=True))
    options = rd.CaptureOptions()
    options.apiValidation = False
    options.debugOutputMute = False
    result = rd.ExecuteAndInject(str(executable), str(root),
        f'--gpu 7900 --authority-test --authority-alpha {alpha} --frames 180 --no-validation',
        [], str(output / 'authority'), options, False)
    require(result.result)
    target = rd.CreateTargetControl('', result.ident, 'Theia authority qualification', True)
    if target is None:
        raise RuntimeError('Cannot connect to capture target')
    capture = None
    try:
        target.QueueCapture(10, 1)
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
        rectangles = reference['actors'] + [reference['obstacle']]
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
        if min(counts) < 10:
            raise RuntimeError(f'Insufficient actor/obstacle/background coverage: {counts}')
        save = rd.TextureSave()
        save.resourceId = color.resourceId
        save.destType = rd.FileType.PNG
        require(controller.SaveTexture(save, str(output/'replay.png')))
        report.update(capture=capture, alpha=alpha, mesh_groups=[3,1,1], sampled_pixels=checked,
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
    (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
os._exit(code)
