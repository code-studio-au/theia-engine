"""Run with RenderDoc 1.46: qrenderdoc --python tools/renderdoc_qualify.py.

Run from the repo root in a desktop session. This exits its dedicated qrenderdoc
process after capturing/replaying; do not paste it into an existing UI session.
See docs/DEVELOPMENT.md for the local Vulkan layer and environment setup.
"""
import json
import hashlib
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
    lod_fine=os.environ.get('THEIA_CAPTURE_LOD_FINE')
    lod_coarse=os.environ.get('THEIA_CAPTURE_LOD_COARSE')
    lod_pixels=os.environ.get('THEIA_CAPTURE_LOD_PIXELS','6')
    lod_hysteresis=os.environ.get('THEIA_CAPTURE_LOD_HYSTERESIS','0')
    lod_trajectory=os.environ.get('THEIA_CAPTURE_LOD_HYSTERESIS_TEST')=='1'
    if bool(lod_fine)!=bool(lod_coarse): raise RuntimeError('LOD capture needs both pages')
    uv_page = os.environ.get('THEIA_CAPTURE_UV_PAGE')
    authored = os.environ.get('THEIA_CAPTURE_MATERIAL_PAGE')
    texture_page = os.environ.get('THEIA_CAPTURE_TEXTURE_PAGE')
    texture_mip_level = os.environ.get('THEIA_CAPTURE_TEXTURE_MIP_LEVEL')
    if texture_mip_level is not None:
        texture_mip_level=int(texture_mip_level)
        if texture_mip_level<0 or not texture_page or os.environ.get('THEIA_CAPTURE_BUNDLE'):
            raise RuntimeError('Fixed mip capture requires an explicit texture page without bundle')
    texture_table = os.environ.get('THEIA_CAPTURE_TEXTURE_TABLE')
    if texture_page and texture_table: raise RuntimeError('Choose single texture or indexed table capture')
    if (texture_page or texture_table) and not (authored and uv_page): raise RuntimeError('Texture capture requires factor and UV pages')
    if authored and uv_page and not (texture_page or texture_table): raise RuntimeError('Choose authored factors or UV diagnostic capture')
    material = os.environ.get('THEIA_CAPTURE_MATERIAL') == '1' or bool(authored) or bool(uv_page)
    cull = os.environ.get('THEIA_CAPTURE_CULL') == '1' or material or bool(lod_fine)
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
    if uv_page:
        if cell_path: raise RuntimeError('UV capture requires fixed geometry')
        uv_page = str(Path(uv_page).resolve())
        quoted_uv = subprocess.list2cmdline([uv_page]) if os.name == 'nt' else shlex.quote(uv_page)
        arguments += f' --uv-test {quoted_uv}'
        if texture_page or texture_table:
            authored = str(Path(authored).resolve())
            selected_texture = str(Path(texture_page or texture_table).resolve())
            if texture_table: texture_table = selected_texture
            else: texture_page = selected_texture
            quoted_material = subprocess.list2cmdline([authored]) if os.name == 'nt' else shlex.quote(authored)
            quoted_texture = subprocess.list2cmdline([selected_texture]) if os.name == 'nt' else shlex.quote(selected_texture)
            texture_option = '--texture-table' if texture_table else '--texture-mip-test' if texture_mip_level is not None else '--texture-test'
            arguments += f' --materials {quoted_material} {texture_option} {quoted_texture}'
            if texture_mip_level is not None: arguments+=f' {texture_mip_level}'
    elif authored:
        if cell_path: raise RuntimeError('Authored material capture requires a fixed geometry page')
        authored = str(Path(authored).resolve())
        quoted_material = subprocess.list2cmdline([authored]) if os.name == 'nt' else shlex.quote(authored)
        arguments += f' --materials {quoted_material}'
    elif material:
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
    bundle_path = os.environ.get('THEIA_CAPTURE_BUNDLE')
    if bundle_path:
        if not (asset_path and authored and uv_page and (texture_page or texture_table)) or cell_path or os.environ.get('THEIA_CAPTURE_FALLBACK') or os.environ.get('THEIA_CAPTURE_BUDGET'):
            raise RuntimeError('Bundle capture requires all expected fixed dependency paths without cell/fallback')
        bundle_path = str(Path(bundle_path).resolve())
        encoded = Path(bundle_path).read_bytes()
        if len(encoded)!=176 or encoded[:16]!=b'THEIABND'+struct.pack('<2I',2 if texture_table else 1,4) or Path(bundle_path).stem!=hashlib.sha256(encoded).hexdigest():
            raise RuntimeError('Invalid expected bundle manifest')
        for i,path in enumerate([asset_path,authored,uv_page,texture_table or texture_page]):
            child=Path(path).read_bytes()
            if encoded[16+i*40:48+i*40]!=hashlib.sha256(child).digest() or struct.unpack_from('<Q',encoded,48+i*40)[0]!=len(child):
                raise RuntimeError('Bundle/expected dependency mismatch')
        initial=os.environ.get('THEIA_CAPTURE_INITIAL_BUNDLE',bundle_path)
        stream=os.environ.get('THEIA_CAPTURE_STREAM_BUNDLE')
        if initial!=bundle_path and not stream:
            raise RuntimeError('Different initial bundle requires streaming capture')
        quote=lambda path: subprocess.list2cmdline([str(Path(path).resolve())]) if os.name=='nt' else shlex.quote(str(Path(path).resolve()))
        arguments=f'--frames 180 --gpu 7900 --no-validation --bundle {quote(initial)} --work-items {scenario}'
        if stream:
            arguments+=f' --stream-bundle {quote(stream)}'
            report['streaming_bundle_capture']=True
            report['initial_bundle_content_id']=Path(initial).stem
            for setting,option in [('THEIA_CAPTURE_BUNDLE_GPU_BUDGET','--bundle-gpu-budget'),
                                   ('THEIA_CAPTURE_BUNDLE_BYTE_BUDGET','--bundle-byte-budget'),
                                   ('THEIA_CAPTURE_TEXTURE_BUDGET','--texture-budget')]:
                value=os.environ.get(setting)
                if value is not None:
                    budget=int(value)
                    if budget<=0: raise RuntimeError('Capture budgets must be positive')
                    arguments+=f' {option} {budget}'
        if shader_dir: arguments+=f' --shader-dir {quoted_dir}'
        report['bundle_dependencies_verified']=True
        report['bundle_content_id']=Path(bundle_path).stem
    if lod_fine:
        if cell_path or bundle_path or authored or uv_page or material:
            raise RuntimeError('LOD capture is a separate two-page gradient/depth path')
        lod_fine=str(Path(lod_fine).resolve());lod_coarse=str(Path(lod_coarse).resolve())
        if asset_path not in [lod_fine,lod_coarse]: raise RuntimeError('Expected LOD asset must be one of the two pages')
        quote=lambda path: subprocess.list2cmdline([path]) if os.name=='nt' else shlex.quote(path)
        arguments=f'--frames 180 --gpu 7900 --no-validation --lod-test {quote(lod_fine)} {quote(lod_coarse)} --work-items {scenario}'
        if lod_trajectory: arguments+=' --lod-hysteresis-test'
        else: arguments+=f' --lod-pixels {float(lod_pixels)} --lod-hysteresis {float(lod_hysteresis)}'
        if shader_dir: arguments+=f' --shader-dir {quoted_dir}'
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
            cull_reference = json.loads(subprocess.check_output([str(executable), '--frustum-ids', asset_path,
                                        str(color.width),str(color.height),str(scenario)],cwd=root).splitlines()[-1])
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
            if lod_fine:
                selected=struct.unpack_from('<I',data,44)[0]
                gpu_pixels=struct.unpack_from('<2f',data,48)
                reason=struct.unpack_from('<I',data,56)[0]
                previous=struct.unpack_from('<I',data,16)[0]
                entry_budget,hard_budget=struct.unpack_from('<2f',data,20)
                reset=struct.unpack_from('<I',data,28)[0]
                if selected not in (0,1) or [lod_fine,lod_coarse][selected]!=asset_path:
                    raise RuntimeError('GPU LOD selection differs from expected resident page')
                if struct.unpack_from('<I',data,32)[0]!=meshlets:
                    raise RuntimeError('GPU selected page meshlet count differs from header')
                oracle=json.loads(subprocess.check_output([str(executable),'--lod-reference',lod_fine,lod_coarse,
                    str(color.width),str(color.height),str(scenario),str(hard_budget)],cwd=root).splitlines()[-1])
                if selected==1 and oracle['level']!='Coarse':
                    raise RuntimeError('GPU chose coarse while conservative CPU model requires fine')
                if oracle['bound_pixels'] is not None and reason in (0,2,4):
                    if any(not math.isfinite(v) or v+1e-6<want for v,want in zip(gpu_pixels,oracle['bound_pixels'])):
                        raise RuntimeError('GPU projected bound falls below CPU model')
                if (selected==1)!=(reason==0) or (selected==1 and any(v>hard_budget for v in gpu_pixels)):
                    raise RuntimeError('GPU LOD reason/budget inconsistent')
                fraction=.2 if lod_trajectory else float(lod_hysteresis)
                if not all(math.isfinite(v) for v in [entry_budget,hard_budget]) or hard_budget<0 or not 0<=entry_budget<=hard_budget*(1-fraction):
                    raise RuntimeError('GPU LOD entry threshold exceeds configured margin')
                if reset not in (0,1) or previous not in (0,1) or (reset==1 and previous!=0):
                    raise RuntimeError('GPU LOD history/reset is invalid')
                if fraction>0 and reason in (0,2,4):
                    eligible=all(v<=hard_budget for v in gpu_pixels)
                    expected=eligible and (previous==1 or all(v<=entry_budget for v in gpu_pixels))
                    if (selected==1)!=expected or (reason==4)!=(eligible and not expected):
                        raise RuntimeError('GPU LOD hysteresis differs from independent threshold state machine')
                if fraction==0 and reason==4:
                    raise RuntimeError('Disabled hysteresis changed stateless policy')
                for path in [lod_fine,lod_coarse]:
                    encoded=Path(path).read_bytes()
                    candidates=[b for b in controller.GetBuffers() if b.length==len(encoded)]
                    if not any(controller.GetBufferData(b.resourceId,0,len(encoded))==encoded for b in candidates):
                        raise RuntimeError('One resident LOD GPU page differs from encoded source')
                report.update(lod_gpu_verified=True,lod_selected=selected,lod_reason=reason,
                    lod_previous=previous,lod_reset=reset,lod_entry_budget=entry_budget,
                    lod_hard_budget=hard_budget,lod_hysteresis_fraction=fraction,
                    lod_gpu_bound_pixels=gpu_pixels,lod_cpu=oracle,lod_both_pages_gpu_verified=True)
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
            if camera and os.environ.get('THEIA_CAPTURE_LOD_GUARD_ONLY')=='1':
                if not lod_fine or report.get('lod_reason')!=1:
                    raise RuntimeError('Guard-only capture requires a verified clip-boundary LOD case')
                report['lod_framebuffer_verified']=False
                report['lod_guard_only']=True
            elif camera:
                verify_camera(controller, executable, root, present.copyDestination, asset_path, scenario, report, cull, material, authored, uv_page, texture_page, texture_table, texture_mip_level)
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


def verify_camera(controller, executable, root, color_id, asset_path, scenario, report, cull=False, material=False, authored=None, uv_page=None, texture_page=None, texture_table=None, texture_mip_level=None):
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
    points = [(int(color.width*x/10), int(color.height*y/10)) for y in range(1,10) for x in range(1,10)]
    if texture_table:
        encoded=Path(texture_table).read_bytes()
        count=struct.unpack_from('<I',encoded,20)[0]
        start=88+((struct.unpack_from('<I',encoded,12)[0]+3)&~3)
        verified=[]
        for i in range(count):
            digest=encoded[start+i*40:start+i*40+32].hex()
            size=struct.unpack_from('<Q',encoded,start+i*40+32)[0]
            data=(Path(texture_table).parent/(digest+'.ktx2')).read_bytes()
            if len(data)!=size or hashlib.sha256(data).hexdigest()!=digest: raise RuntimeError('Expected texture dependency hash/length mismatch')
            width,height=struct.unpack_from('<2I',data,20);offset,length,_=struct.unpack_from('<3Q',data,80)
            matches=[tex for tex in textures if tex.resourceId not in verified and tex.width==width and tex.height==height and controller.GetTextureData(tex.resourceId,rd.Subresource())==data[offset:offset+length]]
            if not matches: raise RuntimeError('Expected complete indexed texture GPU texels')
            verified.append(matches[0].resourceId)
        report['texture_set_gpu_texels_verified']=True
        report['texture_count']=count
    if texture_page:
        data = Path(texture_page).read_bytes()
        width,height=struct.unpack_from('<2I',data,20)
        offset,size,_=struct.unpack_from('<3Q',data,80)
        matches=[tex for tex in textures if tex.width==width and tex.height==height and controller.GetTextureData(tex.resourceId,rd.Subresource())==data[offset:offset+size]]
        if len(matches)!=1: raise RuntimeError('Expected exact resident texture GPU texels')
        if texture_mip_level is not None:
            count=struct.unpack_from('<I',data,40)[0]
            texture=matches[0]
            if texture.mips!=count or texture_mip_level>=count:
                raise RuntimeError('GPU mip count/fixed view is outside the complete chain')
            for level in range(count):
                start,length,_=struct.unpack_from('<3Q',data,80+level*24)
                subresource=rd.Subresource();subresource.mip=level
                if controller.GetTextureData(texture.resourceId,subresource)!=data[start:start+length]:
                    raise RuntimeError('GPU mip texels differ from complete authored source')
            report.update(texture_gpu_mips_verified=True,texture_mip_count=count,
                          texture_fixed_view_level=texture_mip_level)
        report['texture_gpu_texels_verified']=True
        report['texture_dimensions']=[width,height]
    if authored or uv_page:
        for path,key in [(authored,'material_gpu_bytes_verified'),(uv_page,'uv_gpu_bytes_verified'),(texture_table,'texture_table_gpu_bytes_verified')]:
            if not path: continue
            data = Path(path).read_bytes()
            candidates = [buffer for buffer in controller.GetBuffers() if buffer.length == len(data)]
            matches = [buffer for buffer in candidates if controller.GetBufferData(buffer.resourceId,0,len(data)) == data]
            if len(matches) != 1: raise RuntimeError('Expected exact immutable resolve GPU bytes')
            report[key] = True
        probes = {}
        fallback_probes = {}
        for y in range(color.height):
            for x in range(color.width):
                offset = y*color.width+x
                value = struct.unpack_from('<2I',identity,offset*8)
                if value[0]:
                    fallback_probes.setdefault(value, (x,y))
                    if value not in probes and 0 < x < color.width-1 and 0 < y < color.height-1 and all(struct.unpack_from('<2I',identity,neighbor*8) == value for neighbor in (offset-1,offset+1,offset-color.width,offset+color.width)):
                        probes[value] = (x,y)
        report['interior_triangle_probes'] = len(probes)
        for value, pixel in fallback_probes.items():
            probes.setdefault(value, pixel)
        points += list(probes.values())
        report['visible_triangle_probes'] = [list(value) for value in sorted(probes)]
    samples = []
    for pixel in points:
        reference = [str(executable), '--texture-table-reference', asset_path, authored, uv_page, texture_table] if texture_table else [str(executable), '--texture-reference', asset_path, authored, uv_page, texture_page] if texture_page else [str(executable), '--uv-reference', asset_path, uv_page] if uv_page else [str(executable), '--authored-reference', asset_path, authored] if authored else [str(executable), '--material-reference' if material else '--frustum-reference' if cull else '--camera-reference', asset_path]
        if texture_mip_level is not None:
            reference=[str(executable),'--texture-mip-reference',asset_path,authored,uv_page,texture_page,str(texture_mip_level)]
        expected = json.loads(subprocess.check_output(reference + [
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
        sample = {'pixel':pixel,'covered':expected['covered'],'color':actual_color,'depth':actual_depth}
        if material: sample['identity'] = actual_identity
        if uv_page: sample['uv'] = expected['uv']
        if texture_table:
            sample['material_id']=expected['material_id'];sample['texture_index']=expected['texture_index']
        samples.append(sample)
    report.update(camera_samples=samples, camera_verified=True)
    if material: report['material_verified'] = True
    if authored: report['authored_material_verified'] = True
    if uv_page: report['uv_verified'] = True
    if texture_page: report['texture_sampling_verified'] = True
    if texture_table: report['texture_table_sampling_verified'] = True

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
