"""Run with RenderDoc 1.46: qrenderdoc --python tools/renderdoc_qualify.py.

Run from the repo root in a desktop session. This exits its dedicated qrenderdoc
process after capturing/replaying; do not paste it into an existing UI session.
See docs/DEVELOPMENT.md for the local Vulkan layer and environment setup.
"""
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import time
import traceback

import renderdoc as rd


def require_success(result):
    if result != rd.ResultCode.Succeeded:
        raise RuntimeError(str(result))


def walk(actions):
    for action in actions:
        yield action
        yield from walk(action.children)


def qualify(report):
    root = Path.cwd()
    output = Path(os.environ.get('THEIA_CAPTURE_OUTPUT', 'build/qualification/renderdoc')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    executable = root / 'apps/theia-render-lab/target/debug/theia-render-lab'
    if os.name == 'nt':
        executable = executable.with_suffix('.exe')
    options = rd.CaptureOptions()
    # Capture is separate from standalone Vulkan validation qualification. RenderDoc
    # changes presentation layouts and suppresses the callback check when output is muted.
    options.apiValidation = False
    options.debugOutputMute = False
    spatial = os.environ.get('THEIA_CAPTURE_SPATIAL') == '1'
    scenario = int(os.environ.get('THEIA_CAPTURE_SCENARIO', '2'))
    if scenario not in (0, 1, 2):
        raise RuntimeError('Capture scenario must be 0, 1 or 2')
    arguments = '--frames 180 --gpu 7900 --no-validation'
    if spatial:
        arguments += f' --spatial-test --work-items {scenario}'
    result = rd.ExecuteAndInject(
        str(executable), str(root), arguments, [],
        str(output / 'theia-indirect'), options, False)
    require_success(result.result)
    target = rd.CreateTargetControl('', result.ident, 'Theia qualification', True)
    if target is None:
        raise RuntimeError('Could not connect to capture target')
    capture = None
    try:
        target.QueueCapture(10, 1)
        deadline = time.monotonic() + 30
        while target.Connected() and time.monotonic() < deadline:
            message = target.ReceiveMessage(None)
            if message.type == rd.TargetControlMessageType.NewCapture:
                capture = message.newCapture.path
        if target.Connected():
            raise RuntimeError('Target did not exit within 30 seconds')
        if capture is None:
            raise RuntimeError('No capture received; check local Vulkan layer registration')
    finally:
        target.Shutdown()
    report['capture'] = capture
    cap = rd.OpenCaptureFile()
    controller = None
    try:
        require_success(cap.OpenFile(capture, '', None))
        result, controller = cap.OpenCapture(rd.ReplayOptions(), None)
        require_success(result)
        actions = list(walk(controller.GetRootActions()))
        report['actions'] = [
            {'event': action.eventId, 'name': action.GetName(controller.GetStructuredFile()),
             'flags': str(action.flags)} for action in actions]
        if not any(action.flags & rd.ActionFlags.Dispatch for action in actions):
            raise RuntimeError('Missing compute dispatch')
        if not any('vkCmdDrawMeshTasksIndirectCountEXT' in action['name']
                   for action in report['actions']):
            raise RuntimeError('Missing indirect count draw')
        mesh = [action for action in actions if action.flags & rd.ActionFlags.MeshDispatch
                and not action.flags & rd.ActionFlags.MultiAction]
        if len(mesh) != (0 if spatial and scenario == 0 else 1):
            raise RuntimeError('Expected one indirect task dispatch')
        event = mesh[0] if mesh else next(action for action in actions if action.flags & rd.ActionFlags.Dispatch)
        controller.SetFrameEvent(event.eventId, True)
        size = 9728 if spatial else 64
        work = [buffer for buffer in controller.GetBuffers() if buffer.length == size]
        if len(work) != 1:
            raise RuntimeError('Expected one qualification work buffer')
        # Diagnostic replay readback only. The renderer still never reads the count on the host.
        data = controller.GetBufferData(work[0].resourceId, 0, size)
        command = struct.unpack_from('<4I', data)
        report['command_and_count'] = command
        if spatial:
            expected = json.loads(subprocess.check_output([str(executable), '--spatial-reference', str(scenario)], cwd=root))
            count, mask, error, memberships = struct.unpack_from('<4I', data, 16)
            if count > 32 or memberships > 2048:
                raise RuntimeError('GPU visibility or membership capacity exceeded')
            visible = list(struct.unpack_from(f'<{count}I', data, 32))
            offsets = list(struct.unpack_from('<65I', data, 1184))
            members = list(struct.unpack_from(f'<{memberships}I', data, 1444))
            counts = list(struct.unpack_from('<64I', data, 928))
            bounds = list(struct.unpack_from('<192I', data, 160))
            actual_bounds = struct.unpack('<192f', struct.pack('<192I', *bounds))
            expected_bounds = struct.unpack('<192f', struct.pack('<192I', *expected['bounds_words']))
            report['bounds_max_absolute_error'] = max(abs(a-b) for a,b in zip(actual_bounds, expected_bounds))
            # Independently folded CPU/GPU fixture constants may differ by one float ULP.
            if not all(math.isclose(a, b, rel_tol=0, abs_tol=1e-6) for a,b in zip(actual_bounds, expected_bounds)):
                raise RuntimeError('GPU resident bounds differ from CPU reference')
            report.update(scenario=scenario, visible=visible, candidate_mask=mask, membership_count=memberships)
            if error or mask != expected['mask'] or sorted(visible) != expected['visible']:
                raise RuntimeError('GPU visibility/candidates differ from CPU reference')
            if offsets != expected['offsets'] or members != expected['members']:
                raise RuntimeError('GPU CSR differs from CPU reference')
            if counts != [offsets[index+1] - offsets[index] for index in range(64)]:
                raise RuntimeError('GPU CSR counts inconsistent')
            draws = int(count != 0)
            if command != (draws, 1, 1, draws):
                raise RuntimeError('GPU indirect command/count differs from visibility')
        else:
            verify_indirect(data, command, report)
        present = next(action for action in actions if action.flags & rd.ActionFlags.Present)
        controller.SetFrameEvent(present.eventId, True)
        save = rd.TextureSave()
        save.resourceId = present.copyDestination
        save.destType = rd.FileType.PNG
        image = output / 'replay.png'
        require_success(controller.SaveTexture(save, str(image)))
        report['image'] = str(image)
        report['replay_verified'] = True
    finally:
        if controller is not None:
            controller.Shutdown()
        cap.Shutdown()


def verify_indirect(data, command, report):
    mesh_count = struct.unpack_from('<I', data, 36)[0]
    transforms = struct.unpack_from('<5f', data, 16)
    report['command_and_count'] = command
    report['downstream_mesh_count'] = mesh_count
    report['transforms'] = transforms
    if command != (1, 1, 1, 1) or mesh_count != 2:
        raise RuntimeError('Unexpected GPU work descriptor')
    expected = (-0.5, 0.0, 0.5, 0.0, 0.45)
    if any(abs(actual - want) > 1e-6 for actual, want in zip(transforms, expected)):
        raise RuntimeError('Unexpected GPU transforms')

report = {'renderdoc': rd.GetVersionString(), 'validation_enabled_in_capture': False}
code = 0
try:
    qualify(report)
except Exception:
    report['error'] = traceback.format_exc()
    code = 1
finally:
    destination = Path(os.environ.get('THEIA_CAPTURE_OUTPUT', 'build/qualification/renderdoc'))
    destination.mkdir(parents=True, exist_ok=True)
    (destination / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
# qrenderdoc redirects Python stdout to its UI. The report file and process exit
# code are the automation contract. All target/controller/capture handles are closed.
os._exit(code)
