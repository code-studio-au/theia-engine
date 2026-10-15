"""Owned RenderDoc process: verify a cooked quadrant on the benchmark camera route."""
import json
import math
import os
from pathlib import Path
import shlex
import struct
import subprocess
import time
import traceback
import renderdoc as rd


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def success(result):
    require(result == rd.ResultCode.Succeeded, str(result))


def walk(actions):
    for action in actions:
        yield action
        yield from walk(action.children)


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return [x - y for x, y in zip(a, b)]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def decode(cook):
    blobs = [Path(r['path']).read_bytes() for r in [cook, cook['materials'], cook['uv'], cook['texture_table']]]
    mesh, material, uv, _ = blobs
    meta, count, clusters, refs = struct.unpack_from('<4I', mesh, 12)
    offset = 76 + ((meta + 3) & ~3)
    positions = list(struct.iter_unpack('<3f', mesh[offset:offset + count * 12]))
    cluster_offset = offset + count * 12
    refs_offset = cluster_offset + clusters * 16
    triangles_offset = refs_offset + refs * 4
    global_refs = struct.unpack_from(f'<{refs}I', mesh, refs_offset)
    meta, materials = struct.unpack_from('<2I', material, 12)
    assignments_offset = 96 + ((meta + 3) & ~3) + materials * 16 + (clusters + 1) * 4
    meta, count = struct.unpack_from('<2I', uv, 12)
    offset = 88 + ((meta + 3) & ~3)
    texcoords = list(struct.iter_unpack('<2f', uv[offset:offset + count * 8]))
    triangles, hulls = [], []
    for cluster in range(clusters):
        vo, to, vc, tc = struct.unpack_from('<4I', mesh, cluster_offset + cluster * 16)
        hulls.append([positions[global_refs[vo + i]] for i in range(vc)])
        for primitive in range(tc):
            indices = [global_refs[vo + mesh[triangles_offset + to + primitive * 3 + i]] for i in range(3)]
            assignment = struct.unpack_from('<I', material, assignments_offset + len(triangles) * 4)[0]
            triangles.append((cluster + 1, primitive, [positions[i] for i in indices],
                              [texcoords[i] for i in indices], assignment))
    pixels = []
    for record in cook['textures']:
        image = Path(record['path']).read_bytes()
        offset, size = struct.unpack_from('<2Q', image, 80)
        pixels.append(image[offset:offset + size])
    return blobs, triangles, hulls, pixels


def ray(rows, triangles, x, y, width, height):
    basis = [rows[i][:3] for i in (0, 1, 3)]
    eye = [sum(-rows[i][3] * rows[i][a] / dot(rows[i][:3], rows[i][:3]) for i in (0, 1, 3)) for a in range(3)]
    ndc = [(x + .5) / width * 2 - 1, (y + .5) / height * 2 - 1]
    direction = [basis[2][a] + sum(ndc[i] * basis[i][a] / dot(basis[i], basis[i]) for i in (0, 1)) for a in range(3)]
    closest, result = float('inf'), (0, 0, 1., None, None)
    for identity, primitive, (a, b, c), uvs, material in triangles:
        e1, e2 = sub(b, a), sub(c, a)
        p = cross(direction, e2)
        det = dot(e1, p)
        if abs(det) < 1e-12:
            continue
        tv = sub(eye, a)
        u = dot(tv, p) / det
        q = cross(tv, e1)
        v = dot(direction, q) / det
        distance = dot(e2, q) / det
        if not (0 <= u <= 1 and 0 <= v and u + v <= 1 and 0 < distance < closest):
            continue
        point = [eye[i] + direction[i] * distance for i in range(3)]
        clip = [dot(row[:3], point) + row[3] for row in rows]
        depth = clip[2] / clip[3]
        if not 0 <= depth < 1:
            continue
        texcoord = [uvs[0][i] * (1 - u - v) + uvs[1][i] * u + uvs[2][i] * v for i in range(2)]
        closest = distance
        result = (identity, primitive, depth, material, texcoord)
    return result


def qualify(report):
    output = Path(os.environ['THEIA_COOKED_CAPTURE_OUTPUT']).resolve()
    output.mkdir(parents=True, exist_ok=True)
    scene = json.loads(Path(os.environ['THEIA_COOKED_SCENE']).read_bytes())
    quadrant = int(os.environ.get('THEIA_COOKED_QUADRANT', '0'))
    frame = int(os.environ.get('THEIA_COOKED_FRAME', '90'))
    require(0 <= quadrant < 4 and 0 <= frame < 181, 'Invalid quadrant/frame')
    whole_world = os.environ.get('THEIA_COOKED_WHOLE_WORLD') == '1'
    cook = scene['whole_world']['cook'] if whole_world else scene['quadrants'][quadrant]['cook']
    blobs, triangles, hulls, texels = decode(cook)
    exe = Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
    args = ['--benchmark-cooked', '--bundle', cook['bundle']['path'], '--frames', '181',
            '--benchmark-warmup', '0', '--benchmark-report', str(output / 'timings.json'),
            '--no-validation', '--gpu', os.environ.get('THEIA_COOKED_GPU', '7900')]
    options = rd.CaptureOptions()
    options.apiValidation = False
    options.debugOutputMute = False
    result = rd.ExecuteAndInject(str(exe), str(Path.cwd()), shlex.join(args), [], str(output / 'scene'), options, False)
    success(result.result)
    target = rd.CreateTargetControl('', result.ident, 'Theia cooked route qualification', True)
    require(target is not None, 'Missing capture target')
    capture = None
    try:
        target.QueueCapture(frame, 1)
        deadline = time.monotonic() + 45
        while target.Connected() and time.monotonic() < deadline:
            message = target.ReceiveMessage(None)
            if message.type == rd.TargetControlMessageType.NewCapture:
                capture = message.newCapture.path
        require(not target.Connected() and capture is not None, 'Capture missing or timed out')
    finally:
        target.Shutdown()
    cap = rd.OpenCaptureFile()
    controller = None
    try:
        success(cap.OpenFile(capture, '', None))
        result, controller = cap.OpenCapture(rd.ReplayOptions(), None)
        success(result)
        actions = list(walk(controller.GetRootActions()))
        draws = [a for a in actions if a.flags & rd.ActionFlags.MeshDispatch and a.flags & rd.ActionFlags.Indirect and not a.flags & rd.ActionFlags.MultiAction]
        require(len(draws) == 1, 'Expected one indirect cooked draw')
        controller.SetFrameEvent(draws[0].eventId, True)
        buffers = controller.GetBuffers()
        for blob in blobs:
            require(any(bytes(controller.GetBufferData(b.resourceId, 0, len(blob))) == blob
                        for b in buffers if b.length == len(blob)), 'Cooked GPU buffer differs from immutable source')
        work = [b for b in buffers if b.length == 256]
        require(len(work) == 1, 'Missing unique cooked work buffer')
        data = controller.GetBufferData(work[0].resourceId, 0, 256)
        count, mask = struct.unpack_from('<2I', data, 36)
        ids = list(struct.unpack_from(f'<{count}I', data, 64))
        present = next(a for a in actions if a.flags & rd.ActionFlags.Present)
        controller.SetFrameEvent(present.eventId, True)
        textures = controller.GetTextures()
        color = next(t for t in textures if t.resourceId == present.copyDestination)
        width, height = color.width, color.height
        reference = json.loads(subprocess.check_output([str(exe), '--benchmark-reference', '0', str(frame), '181', str(width), str(height)]))
        rows = reference['rows']
        visible = []
        for i, hull in enumerate(hulls):
            common = 63
            for point in hull:
                x, y, z, w = [dot(row[:3], point) + row[3] for row in rows]
                common &= int(x < -w) | (int(x > w) << 1) | (int(y < -w) << 2) | (int(y > w) << 3) | (int(z < 0) << 4) | (int(z > w) << 5)
            if common == 0:
                visible.append(i)
        require(sorted(ids) == visible and len(set(ids)) == count and mask == sum(1 << i for i in visible), 'Cooked culling differs from CPU hull oracle')
        images = [t for t in textures if (t.width, t.height) == (64, 64)]
        require(len(images) == 4, 'Missing four resident cooked images')
        require(sorted(bytes(controller.GetTextureData(t.resourceId, rd.Subresource())) for t in images) == sorted(texels), 'GPU texture set differs from native cooked texels')
        identity = next(t for t in textures if t.format.compType == rd.CompType.UInt and t.format.compCount == 2)
        depth = next(t for t in textures if t.format.compType == rd.CompType.Depth)
        identities = controller.GetTextureData(identity.resourceId, rd.Subresource())
        depths = controller.GetTextureData(depth.resourceId, rd.Subresource())
        rgb = controller.GetTextureData(color.resourceId, rd.Subresource())
        limits = {i: sum(t[0] == i for t in triangles) for i in range(1, len(hulls) + 1)}
        covered = 0
        for offset in range(width * height):
            ident, primitive = struct.unpack_from('<2I', identities, offset * 8)
            d = struct.unpack_from('<f', depths, offset * 4)[0]
            require(math.isfinite(d) and 0 <= d <= 1, 'Invalid depth')
            require((ident in limits and ident - 1 in ids and primitive < limits[ident] and d < 1) if ident else d == 1, 'Invalid identity/depth/visible set')
            covered += int(ident != 0)
        require(covered > 0, 'Empty cooked scene')
        samples, materials, boundary_probes = [], set(), 0

        def probe(x, y):
            nonlocal boundary_probes
            ident, primitive, expected_depth, material, uv = ray(rows, triangles, x, y, width, height)
            offset = y * width + x
            actual = struct.unpack_from('<2I', identities, offset * 8)
            d = struct.unpack_from('<f', depths, offset * 4)[0]
            require(actual == (ident, primitive) and abs(d - expected_depth) <= 2e-5, f'Ray/raster identity/depth differs at {x},{y}: {actual} vs {(ident, primitive)}')
            if ident:
                # The bounded f32 interpolation profile permits 0.002 texel of
                # coordinate error at a nearest-cell edge. Admit only its adjacent
                # source texels, never another material or an arbitrary colour.
                indices = [{math.floor(v * 64 + delta) % 64 for delta in (-.002, .002)} for v in uv]
                boundary_probes += int(any(len(values) > 1 for values in indices))
                expected = [list(texels[material][(ty * 64 + tx) * 4:(ty * 64 + tx) * 4 + 3])
                            for tx in indices[0] for ty in indices[1]]
                materials.add(material)
            else:
                expected = [[round((v * 12.92 if v <= .0031308 else 1.055 * v ** (1 / 2.4) - .055) * 255) for v in (.025, .07, .12)]]
            actual_rgb = list(rgb[offset * 4:offset * 4 + 3])
            if color.format.BGRAOrder():
                actual_rgb.reverse()
            require(any(all(abs(a - b) <= 2 for a, b in zip(actual_rgb, candidate)) for candidate in expected), f'Cooked texture colour differs at {x},{y}: {actual_rgb} vs {expected}')
            samples.append({'pixel': [x, y], 'identity': actual, 'material': material, 'color': actual_rgb})

        for yf in (.113, .317, .519, .723, .911):
            for xf in (.113, .317, .519, .723, .911):
                probe(int(width * xf), int(height * yf))
        # Try projected triangle interiors to cover small roofs/trees missed by the grid.
        for _, _, points, _, material in triangles:
            if material in materials:
                continue
            center = [sum(p[a] for p in points) / 3 for a in range(3)]
            clip = [dot(row[:3], center) + row[3] for row in rows]
            if clip[3] <= 0:
                continue
            x, y = [int((clip[a] / clip[3] + 1) * .5 * (width if a == 0 else height)) for a in (0, 1)]
            if 0 <= x < width and 0 <= y < height:
                probe(x, y)
        require(len(samples) >= 20, 'Insufficient stable ray/colour probes')
        save = rd.TextureSave()
        save.resourceId = present.copyDestination
        save.destType = rd.FileType.PNG
        success(controller.SaveTexture(save, str(output / 'preview.png')))
        report.update(passed=True, capture=capture, quadrant=None if whole_world else quadrant, frame=frame,
                      scene_selection='whole-world' if whole_world else f'quadrant-{quadrant}',
                      visible_meshlets=ids, gpu_buffers_verified=4, gpu_texture_texels_verified=16384,
                      whole_image_identity_depth_verified=True, covered_pixels=covered,
                      samples=samples, sampled_materials=sorted(materials), boundary_envelope_probes=boundary_probes, texel_coordinate_envelope=.002)
    finally:
        if controller is not None:
            controller.Shutdown()
        cap.Shutdown()


report = {'passed': False, 'validation_in_capture': False, 'renderdoc': rd.GetVersionString()}
code = 0
try:
    qualify(report)
except Exception:
    report['error'] = traceback.format_exc()
    code = 1
finally:
    output = Path(os.environ['THEIA_COOKED_CAPTURE_OUTPUT'])
    output.mkdir(parents=True, exist_ok=True)
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
os._exit(code)
