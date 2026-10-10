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
import shlex
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
    visibility = os.environ.get('THEIA_CAPTURE_VISIBILITY') == '1'
    asset_path = os.environ.get('THEIA_CAPTURE_ASSET')
    cell_path = os.environ.get('THEIA_CAPTURE_CELL')
    material = os.environ.get('THEIA_CAPTURE_MATERIAL') == '1'
    cull = os.environ.get('THEIA_CAPTURE_CULL') == '1' or material
    camera = os.environ.get('THEIA_CAPTURE_CAMERA') == '1' or cull
    if camera and not asset_path:
        raise RuntimeError('Camera capture requires expected mesh page')
    if cell_path and not asset_path:
        raise RuntimeError('Cell capture requires expected bound page in THEIA_CAPTURE_ASSET')
    if sum([spatial, visibility, bool(asset_path)]) > 1:
        raise RuntimeError('Select only one qualification capture mode')
    scenario = int(os.environ.get('THEIA_CAPTURE_SCENARIO', '2'))
    if scenario not in (0, 1, 2):
        raise RuntimeError('Capture scenario must be 0, 1 or 2')
    arguments = '--frames 180 --gpu 7900 --no-validation'
    shader_dir = os.environ.get('THEIA_CAPTURE_SHADER_DIR')
    if shader_dir:
        shader_dir = str(Path(shader_dir).resolve())
        quoted_dir = subprocess.list2cmdline([shader_dir]) if os.name == 'nt' else shlex.quote(shader_dir)
        arguments += f' --shader-dir {quoted_dir}'
        report['shader_directory'] = shader_dir
    if material:
        arguments += ' --material-test'
    elif cull:
        arguments += ' --frustum-test'
    elif camera:
        arguments += ' --camera-test'
    if spatial:
        arguments += f' --spatial-test --work-items {scenario}'
    if visibility:
        arguments += f' --visibility-test --work-items {scenario}'
    if asset_path:
        asset_path = str(Path(asset_path).resolve())
        quoted = subprocess.list2cmdline([asset_path]) if os.name == 'nt' else shlex.quote(asset_path)
        if cell_path:
            cell_path = str(Path(cell_path).resolve())
            quoted_cell = subprocess.list2cmdline([cell_path]) if os.name == 'nt' else shlex.quote(cell_path)
            arguments += f' --cell {quoted_cell} --work-items {scenario}'
            resident_budget = os.environ.get('THEIA_CAPTURE_RESIDENT_BUDGET')
            if resident_budget:
                arguments += f' --resident-budget {int(resident_budget)}'
        else:
            arguments += f' --asset {quoted} --work-items {scenario}'
        fallback = os.environ.get('THEIA_CAPTURE_FALLBACK')
        budget = os.environ.get('THEIA_CAPTURE_BUDGET')
        if cell_path and (fallback or budget):
            raise RuntimeError('Cell capture uses physical resident budget, not asset fallback arguments')
        if fallback:
            fallback = str(Path(fallback).resolve())
            quoted = subprocess.list2cmdline([fallback]) if os.name == 'nt' else shlex.quote(fallback)
            arguments += f' --fallback {quoted}'
        if budget:
            arguments += f' --asset-budget {int(budget)}'
            if Path(asset_path).stat().st_size > int(budget):
                if not fallback:
                    raise RuntimeError('Capture budget requires explicit fallback')
                asset_path = fallback
    result = rd.ExecuteAndInject(
        str(executable), str(root), arguments, [],
        str(output / 'theia-indirect'), options, False)
    require_success(result.result)
    target = rd.CreateTargetControl('', result.ident, 'Theia qualification', True)
    if target is None:
        raise RuntimeError('Could not connect to capture target')
    capture = None
    try:
        capture_frame = int(os.environ.get('THEIA_CAPTURE_FRAME', '10'))
        if not 0 <= capture_frame < 150:
            raise RuntimeError('Capture frame must be 0..149')
        target.QueueCapture(capture_frame, 1)
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
        cull_reference = None
        if cull:
            present_action = next(action for action in actions if action.flags & rd.ActionFlags.Present)
            color = next(texture for texture in controller.GetTextures() if texture.resourceId == present_action.copyDestination)
            cull_reference = json.loads(subprocess.check_output([str(executable), '--frustum-reference', asset_path,
                                        '0','0',str(color.width),str(color.height),str(scenario)],cwd=root))
        report['actions'] = [
            {'event': action.eventId, 'name': action.GetName(controller.GetStructuredFile()),
             'flags': str(action.flags)} for action in actions]
        if not any(action.flags & rd.ActionFlags.Dispatch for action in actions):
            raise RuntimeError('Missing compute dispatch')
        if not any('vkCmdDrawMeshTasksIndirectCountEXT' in action['name']
                   for action in report['actions']):
            raise RuntimeError('Missing indirect count draw')
        mesh = [action for action in actions if action.flags & rd.ActionFlags.MeshDispatch
                and action.flags & rd.ActionFlags.Indirect
                and not action.flags & rd.ActionFlags.MultiAction]
        expected_dispatches = int(bool(cull_reference['ids'])) if cull else (0 if (spatial or visibility or asset_path) and scenario == 0 else 1)
        if len(mesh) != expected_dispatches:
            raise RuntimeError('Expected one indirect task dispatch')
        event = mesh[0] if mesh else next(action for action in actions if action.flags & rd.ActionFlags.Dispatch)
        controller.SetFrameEvent(event.eventId, True)
        size = 9728 if spatial else 256 if cull else 64
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
        elif asset_path:
            source = Path(asset_path).read_bytes()
            meshlets = struct.unpack_from('<I', source, 20)[0]
            count = len(cull_reference['ids']) if cull else 0 if scenario == 0 else min(meshlets,1) if scenario == 1 else meshlets
            if cull:
                actual_count, mask = struct.unpack_from('<2I',data,36)
                if actual_count > 32:
                    raise RuntimeError('GPU meshlet visibility exceeds qualification capacity')
                visible = list(struct.unpack_from(f'<{actual_count}I',data,64))
                if sorted(visible) != cull_reference['ids'] or mask != sum(1<<id for id in cull_reference['ids']):
                    raise RuntimeError('GPU meshlet frustum IDs/mask differ from CPU oracle')
                report.update(visible_meshlets=visible, visibility_mask=mask, frustum_verified=True)
            draws = int(count != 0)
            if struct.unpack_from('<I', data, 36)[0] != count or command != (draws,1,1,draws):
                raise RuntimeError('Cooked mesh indirect count differs from asset header/scenario')
            arena_size = os.environ.get('THEIA_CAPTURE_ARENA_BYTES')
            arena_offset = int(os.environ.get('THEIA_CAPTURE_ARENA_OFFSET', '0'))
            geometry_size = int(arena_size) if arena_size else len(source)
            if arena_offset < 0 or arena_offset + len(source) > geometry_size:
                raise RuntimeError('Expected geometry range exceeds capture buffer')
            geometry = [buffer for buffer in controller.GetBuffers() if buffer.length == geometry_size]
            if len(geometry) != 1 or controller.GetBufferData(geometry[0].resourceId,arena_offset,len(source)) != source:
                raise RuntimeError('Immutable GPU asset bytes differ from cooked source')
            report.update(scenario=scenario, meshlet_count=count, immutable_asset_bytes_verified=True)
            if cell_path:
                report.update(cell_manifest=cell_path, expected_page=asset_path, capture_frame=capture_frame)
                if arena_size:
                    report.update(geometry_arena_bytes=geometry_size, verified_page_offset=arena_offset)
        elif visibility:
            count = struct.unpack_from('<I', data, 36)[0]
            draws = int(scenario != 0)
            if count != scenario or command != (draws, 1, 1, draws):
                raise RuntimeError('Visibility fixture command/count mismatch')
        else:
            verify_indirect(data, command, report)
        present = next(action for action in actions if action.flags & rd.ActionFlags.Present)
        controller.SetFrameEvent(present.eventId, True)
        if visibility:
            verify_surface(controller, executable, root, present.copyDestination, scenario, report)
        if asset_path:
            if camera:
                verify_camera(controller, executable, root, present.copyDestination, asset_path, scenario, report, cull, material)
            else:
                verify_asset(controller, present.copyDestination, source, count, report)
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


def verify_surface(controller, executable, root, color_id, scenario, report):
    textures = controller.GetTextures()
    color = next(texture for texture in textures if texture.resourceId == color_id)
    identities = [texture for texture in textures if texture.format.compType == rd.CompType.UInt
                  and texture.format.compCount == 2 and texture.format.compByteWidth == 4]
    depths = [texture for texture in textures if texture.format.compType == rd.CompType.Depth
              and texture.format.compByteWidth == 4]
    if len(identities) != 1 or len(depths) != 1:
        raise RuntimeError('Expected one identity and depth image')
    identity = identities[0]
    depth = depths[0]
    width, height = color.width, color.height
    ids = controller.GetTextureData(identity.resourceId, rd.Subresource())
    z = controller.GetTextureData(depth.resourceId, rd.Subresource())
    rgb = controller.GetTextureData(color.resourceId, rd.Subresource())
    if len(ids) != width*height*8 or len(z) != width*height*4 or len(rgb) != width*height*4:
        raise RuntimeError('Unexpected texture readback layout')
    def encode_color(value):
        value = max(0, min(1, value))
        srgb = value*12.92 if value <= 0.0031308 else 1.055*value**(1/2.4)-0.055
        return round(srgb*255)
    points = [(width//2, height//2), (int(width*.725), int(height*.65)), (int(width*.9), int(height*.1))]
    points += [(int(width*x/10), int(height*y/10)) for y in range(1,10) for x in range(1,10)]
    samples = []
    for x,y in points:
        expected = json.loads(subprocess.check_output([str(executable), '--visibility-reference',
                              str(x), str(y), str(width), str(height), str(scenario)], cwd=root))
        offset = y*width+x
        actual_id = struct.unpack_from('<2I', ids, offset*8)
        actual_depth = struct.unpack_from('<f', z, offset*4)[0]
        actual_color = list(rgb[offset*4:offset*4+3])
        if color.format.BGRAOrder():
            actual_color.reverse()
        expected_color = [encode_color(value) for value in expected['color']]
        if list(actual_id) != expected['identity'] or abs(actual_depth-expected['depth']) > 1e-6:
            raise RuntimeError(f'Identity/depth mismatch at {(x,y)}: {actual_id}/{actual_depth}, expected {expected}')
        if any(abs(actual-want) > 2 for actual,want in zip(actual_color, expected_color)):
            raise RuntimeError(f'Material resolve mismatch at {(x,y)}: {actual_color}, expected {expected_color}')
        samples.append({'pixel': [x,y], 'identity': actual_id, 'depth': actual_depth, 'color': actual_color})
    report.update(scenario=scenario, surface_samples=samples, surface_verified=True)


def verify_asset(controller, color_id, source, count, report):
    header = struct.unpack_from('<9I', source,8)
    _, metadata, vertices, meshlets, refs, _, _, _, _ = header
    vertex_offset = 76+((metadata+3)&~3)
    meshlet_offset = vertex_offset+vertices*12
    refs_offset = meshlet_offset+meshlets*16
    triangles_offset = refs_offset+refs*4
    geometry = []
    for index in range(count):
        vo,to,vc,tc = struct.unpack_from('<4I',source,meshlet_offset+index*16)
        for triangle in range(tc):
            ids = source[triangles_offset+to+triangle*3:triangles_offset+to+triangle*3+3]
            global_ids = [struct.unpack_from('<I',source,refs_offset+(vo+local)*4)[0] for local in ids]
            geometry.append([struct.unpack_from('<3f',source,vertex_offset+vertex*12) for vertex in global_ids])
    color = next(texture for texture in controller.GetTextures() if texture.resourceId == color_id)
    rgb = controller.GetTextureData(color_id,rd.Subresource())
    if len(rgb) != color.width*color.height*4:
        raise RuntimeError('Unexpected cooked viewer image layout')
    def inside(point, triangle):
        a,b,c = triangle
        denominator = (b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1])
        if abs(denominator)<1e-12:
            return False
        u = ((b[1]-c[1])*(point[0]-c[0])+(c[0]-b[0])*(point[1]-c[1]))/denominator
        v = ((c[1]-a[1])*(point[0]-c[0])+(a[0]-c[0])*(point[1]-c[1]))/denominator
        return min(u,v,1-u-v)>=0
    identities = [texture for texture in textures if texture.format.compType == rd.CompType.UInt and texture.format.compCount == 2 and texture.format.compByteWidth == 4]
    identity = None
    if material:
        if len(identities) != 1: raise RuntimeError('Expected one cooked identity target')
        identity = controller.GetTextureData(identities[0].resourceId, rd.Subresource())
        if len(identity) != color.width*color.height*8: raise RuntimeError('Unexpected identity layout')
    def srgb(value):
        return round((value*12.92 if value<=.0031308 else 1.055*value**(1/2.4)-.055)*255)
    samples = []
    for y in range(1,10):
        for x in range(1,10):
            pixel = (int(color.width*x/10),int(color.height*y/10))
            point = ((pixel[0]+.5)/color.width*2-1,(pixel[1]+.5)/color.height*2-1)
            covered = any(inside(point,triangle) for triangle in geometry)
            linear = [(point[0]+1)*.5,(point[1]+1)*.5,.5] if covered else [.025,.07,.12]
            expected = [srgb(value) for value in linear]
            offset = (pixel[1]*color.width+pixel[0])*4
            actual = list(rgb[offset:offset+3])
            if color.format.BGRAOrder():
                actual.reverse()
            if any(abs(a-b)>2 for a,b in zip(actual,expected)):
                raise RuntimeError(f'Cooked mesh raster coverage/color mismatch at {pixel}: {actual}, expected {expected}')
            samples.append({'pixel':pixel,'covered':covered,'color':actual})
    report.update(asset_surface_samples=samples, asset_surface_verified=True, selected_triangles=len(geometry))


def verify_camera(controller, executable, root, color_id, asset_path, scenario, report, cull=False, material=False):
    textures = controller.GetTextures()
    color = next(texture for texture in textures if texture.resourceId == color_id)
    depths = [texture for texture in textures if texture.format.compType == rd.CompType.Depth
              and texture.format.compByteWidth == 4]
    if len(depths) != 1:
        raise RuntimeError('Expected one camera depth target')
    rgb = controller.GetTextureData(color_id, rd.Subresource())
    depth = controller.GetTextureData(depths[0].resourceId, rd.Subresource())
    if len(rgb) != color.width*color.height*4 or len(depth) != color.width*color.height*4:
        raise RuntimeError('Unexpected camera readback layout')
    identities = [texture for texture in textures if texture.format.compType == rd.CompType.UInt and texture.format.compCount == 2 and texture.format.compByteWidth == 4]
    identity = None
    if material:
        if len(identities) != 1: raise RuntimeError('Expected one cooked identity target')
        identity = controller.GetTextureData(identities[0].resourceId, rd.Subresource())
        if len(identity) != color.width*color.height*8: raise RuntimeError('Unexpected identity layout')
    def srgb(value):
        value = max(0, min(1, value))
        return round((value*12.92 if value <= .0031308 else 1.055*value**(1/2.4)-.055)*255)
    samples = []
    for y in range(1,10):
        for x in range(1,10):
            pixel = (int(color.width*x/10), int(color.height*y/10))
            expected = json.loads(subprocess.check_output([str(executable), '--material-reference' if material else '--frustum-reference' if cull else '--camera-reference', asset_path,
                                  str(pixel[0]), str(pixel[1]), str(color.width), str(color.height), str(scenario)], cwd=root))
            offset = pixel[1]*color.width+pixel[0]
            if material:
                actual_identity = list(struct.unpack_from('<2I', identity, offset*8))
                if actual_identity != expected['identity']: raise RuntimeError(f'Cooked identity mismatch at {pixel}: {actual_identity}, expected {expected}')
            actual_color = list(rgb[offset*4:offset*4+3])
            if color.format.BGRAOrder():
                actual_color.reverse()
            actual_depth = struct.unpack_from('<f', depth, offset*4)[0]
            if abs(actual_depth-expected['depth']) > 2e-6 or any(abs(a-srgb(b)) > 2 for a,b in zip(actual_color,expected['color'])):
                raise RuntimeError(f'Perspective camera color/depth mismatch at {pixel}: {actual_color}/{actual_depth}, expected {expected}')
            samples.append({'pixel':pixel,'covered':expected['covered'],'color':actual_color,'depth':actual_depth})
    report.update(camera_samples=samples, camera_verified=True)
    if material: report['material_verified'] = True

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
