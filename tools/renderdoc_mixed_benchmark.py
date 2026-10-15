"""Owned RenderDoc replay: cooked world and visual proxies share visibility/depth."""
import json
import math
import os
from pathlib import Path
import re
import shlex
import struct
import subprocess
import sys
sys.dont_write_bytecode = True
import time
import traceback
import renderdoc as rd
sys.path.insert(0, str(Path.cwd() / 'tools'))
from cooked_benchmark_reference import decode, ray, dot, sub
from proxy_benchmark_reference import pixel_reference, color_reference, encode, conservative_visible

FLAG = 0x80000000


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def success(result):
    require(result == rd.ResultCode.Succeeded, str(result))


def walk(actions):
    for action in actions:
        yield action
        yield from walk(action.children)


def qualify(report):
    output = Path(os.environ['THEIA_MIXED_CAPTURE_OUTPUT']).resolve()
    output.mkdir(parents=True, exist_ok=True)
    scene = json.loads(Path(os.environ['THEIA_MIXED_SCENE']).read_bytes())
    cook = scene['whole_world']['cook']
    blobs, triangles, hulls, texels = decode(cook)
    players = int(os.environ.get('THEIA_MIXED_PLAYERS', '1000'))
    frame = int(os.environ.get('THEIA_MIXED_FRAME', '90'))
    overlay = os.environ.get('THEIA_MIXED_OVERLAY') == '1'
    require(0 <= players <= 10000 and 0 <= frame < 181, 'Invalid bounded capture plan')
    exe = Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
    args = ['--benchmark-mixed', '--bundle', cook['bundle']['path'], '--benchmark-players', str(players),
            '--frames', '181', '--benchmark-warmup', '0', '--benchmark-report', str(output / 'timings.json'),
            '--no-validation', '--gpu', os.environ.get('THEIA_MIXED_GPU', '7900')]
    if not overlay:
        args.append('--benchmark-no-overlay')
    options = rd.CaptureOptions()
    options.apiValidation = False
    options.debugOutputMute = False
    result = rd.ExecuteAndInject(str(exe), str(Path.cwd()), shlex.join(args), [], str(output / 'scene'), options, False)
    success(result.result)
    target = rd.CreateTargetControl('', result.ident, 'Theia mixed benchmark qualification', True)
    require(target is not None, 'Missing capture target')
    capture = None
    try:
        target.QueueCapture(frame, 1)
        deadline = time.monotonic() + 60
        while target.Connected() and time.monotonic() < deadline:
            message = target.ReceiveMessage(None)
            if message.type == rd.TargetControlMessageType.NewCapture:
                capture = message.newCapture.path
        require(not target.Connected() and capture is not None, 'Missing capture or timed out')
    finally:
        target.Shutdown()
    timings = json.loads((output / 'timings.json').read_bytes())
    require(timings['workload'] == 'theia-cooked-mmo-v1' and timings['players'] == players, 'Wrong mixed workload report')
    cap = rd.OpenCaptureFile()
    controller = None
    try:
        success(cap.OpenFile(capture, '', None))
        result, controller = cap.OpenCapture(rd.ReplayOptions(), None)
        success(result)
        actions = list(walk(controller.GetRootActions()))
        draws = [a for a in actions if a.flags & rd.ActionFlags.MeshDispatch and a.flags & rd.ActionFlags.Indirect and not a.flags & rd.ActionFlags.MultiAction]
        require(len(draws) == 2, 'Expected two fixed indirect mesh draws')
        controller.SetFrameEvent(draws[-1].eventId, True)
        buffers = controller.GetBuffers()
        for blob in blobs:
            require(any(bytes(controller.GetBufferData(b.resourceId, 0, len(blob))) == blob for b in buffers if b.length == len(blob)), 'Cooked GPU buffer differs')
        static_work = [b for b in buffers if b.length == 256]
        proxy_work = [b for b in buffers if b.length == 376160]
        require(len(static_work) == len(proxy_work) == 1, 'Missing independent work buffers')
        static = controller.GetBufferData(static_work[0].resourceId, 0, 256)
        dynamic = controller.GetBufferData(proxy_work[0].resourceId, 0, 376160)
        count, y, z, draws_count, total, near, far, visual = struct.unpack_from('<8I', dynamic)
        require((y, z, draws_count, total) == (1, 1, 1, players) and count <= players, 'Invalid proxy work header')
        present = next(a for a in actions if a.flags & rd.ActionFlags.Present)
        controller.SetFrameEvent(present.eventId, True)
        textures = controller.GetTextures()
        color = next(t for t in textures if t.resourceId == present.copyDestination)
        width, height = color.width, color.height
        reference = json.loads(subprocess.check_output([str(exe), '--benchmark-mixed-reference', str(players), str(frame), '181', str(width), str(height)]))
        rows = reference['rows']
        expected_visible, objects, maximum = [], {}, 0.
        for obj in reference['objects']:
            ident = obj['id']
            record = struct.unpack_from('<3fI3fI', dynamic, 41824 + ident * 32)
            error = max(abs(a - b) for a, b in zip(record[:3], obj['center']))
            maximum = max(maximum, error)
            require(error <= .001 and record[3] == FLAG | 3 and all(abs(a - b) <= 1e-6 for a, b in zip(record[4:7], obj['extent'])), 'Proxy pose/namespace/extent differs')
            actual_near = dot(sub(record[:3], reference['eye']), sub(record[:3], reference['eye'])) < 35 * 35
            require(bool(record[7]) == actual_near, 'Proxy detail rule differs')
            if conservative_visible(rows, record[:3], record[4:7]):
                expected_visible.append(ident)
            objects[ident] = (ident, record[:3], record[4:7], bool(record[7]))
        ids = list(struct.unpack_from(f'<{count}I', dynamic, 32))
        require(sorted(ids) == expected_visible and len(set(ids)) == count, 'Proxy culling/compaction differs')
        require(near == sum(objects[i][3] for i in ids) and near + far == count and visual == count, 'Proxy counters differ')
        static_ids = []
        for i, hull in enumerate(hulls):
            common = 63
            for point in hull:
                x, yy, zz, w = [dot(row[:3], point) + row[3] for row in rows]
                common &= int(x < -w) | (int(x > w) << 1) | (int(yy < -w) << 2) | (int(yy > w) << 3) | (int(zz < 0) << 4) | (int(zz > w) << 5)
            if common == 0:
                static_ids.append(i)
        static_count, mask = struct.unpack_from('<2I', static, 36)
        require(sorted(struct.unpack_from(f'<{static_count}I', static, 64)) == static_ids and mask == sum(1 << i for i in static_ids), 'Static compaction differs')
        images = [t for t in textures if (t.width, t.height) == (64, 64)]
        require(len(images) == 4 and sorted(bytes(controller.GetTextureData(t.resourceId, rd.Subresource())) for t in images) == sorted(texels), 'Cooked texture set differs')
        identity = next(t for t in textures if t.format.compType == rd.CompType.UInt and t.format.compCount == 2)
        depth = next(t for t in textures if t.format.compType == rd.CompType.Depth)
        identities = controller.GetTextureData(identity.resourceId, rd.Subresource())
        depths = controller.GetTextureData(depth.resourceId, rd.Subresource())
        rgb = controller.GetTextureData(color.resourceId, rd.Subresource())
        limits = {i: sum(t[0] == i for t in triangles) for i in range(1, len(hulls) + 1)}
        proxy_set, static_set = set(ids), set(static_ids)
        world_pixels = proxy_pixels = 0
        for offset in range(width * height):
            ident, primitive = struct.unpack_from('<2I', identities, offset * 8)
            d = struct.unpack_from('<f', depths, offset * 4)[0]
            require(math.isfinite(d) and 0 <= d <= 1, 'Invalid mixed depth')
            if not ident:
                require(d == 1, 'Background depth differs')
            elif ident & FLAG:
                index = (ident & ~FLAG) - 1
                require(index in proxy_set and primitive < (16 if objects[index][3] else 12) and d < 1, 'Invalid proxy identity/primitive')
                proxy_pixels += 1
            else:
                require(ident - 1 in static_set and primitive < limits[ident] and d < 1, 'Invalid static identity/primitive')
                world_pixels += 1
        require(world_pixels > 0 and (proxy_pixels > 0 if players else proxy_pixels == 0), 'Mixed image lacks expected populations')
        candidates = [objects[i] for i in ids]
        samples = []
        occlusion = {'world_in_front': 0, 'proxy_in_front': 0}
        boundaries = 0

        def probe(x, yy):
            nonlocal boundaries
            if overlay and 12 <= x < 420 and 12 <= yy < 300:
                return
            world = ray(rows, triangles, x, yy, width, height)
            proxy = pixel_reference(rows, candidates, x, yy, width, height)
            if world[0] and proxy[0]:
                occlusion['proxy_in_front' if proxy[2] < world[2] else 'world_in_front'] += 1
            if proxy[0] and proxy[2] < world[2]:
                expected_id, primitive, expected_depth = proxy
                expected_id |= FLAG
                expected = [list(map(encode, color_reference(proxy[0], primitive, expected_depth, 3)))]
            else:
                expected_id, primitive, expected_depth, material, uv = world
                if expected_id:
                    coordinates = [{math.floor(v * 64 + delta) % 64 for delta in (-.002, .002)} for v in uv]
                    boundaries += int(any(len(v) > 1 for v in coordinates))
                    expected = [list(texels[material][(ty * 64 + tx) * 4:(ty * 64 + tx) * 4 + 3]) for tx in coordinates[0] for ty in coordinates[1]]
                else:
                    expected = [list(map(encode, [.025, .07, .12]))]
            offset = yy * width + x
            actual = struct.unpack_from('<2I', identities, offset * 8)
            d = struct.unpack_from('<f', depths, offset * 4)[0]
            require(actual == (expected_id, primitive) and abs(d - expected_depth) <= 2e-5, f'Mixed ray/raster differs at {x},{yy}: {actual} vs {(expected_id, primitive)}')
            actual_rgb = list(rgb[offset * 4:offset * 4 + 3])
            if color.format.BGRAOrder():
                actual_rgb.reverse()
            require(any(all(abs(a - b) <= 2 for a, b in zip(actual_rgb, e)) for e in expected), f'Mixed deferred colour differs at {x},{yy}: {actual_rgb} vs {expected}')
            samples.append({'pixel': [x, yy], 'identity': actual, 'color': actual_rgb})

        for yf in (.113, .317, .519, .723, .911):
            for xf in (.113, .317, .519, .723, .911):
                probe(int(width * xf), int(height * yf))
        for ident in ids[::max(1, len(ids) // 32)]:
            center = objects[ident][1]
            clip = [dot(row[:3], center) + row[3] for row in rows]
            if clip[3] > 0:
                x, yy = [int((clip[a] / clip[3] + 1) * .5 * (width if a == 0 else height)) for a in (0, 1)]
                if 0 <= x < width and 0 <= yy < height:
                    probe(x, yy)
        require(len(samples) >= 20, 'Insufficient mixed ray probes')
        if overlay:
            recent = timings['samples'][max(0, frame - 30):frame]
            last = timings['samples'][max(0, frame - 1)]
            stats = timings['samples'][frame]['cooked']
            scaled = lambda v, scale: math.floor(v * scale + .5)
            fps = scaled(1000 * len(recent) / sum(s['frame_interval_ms'] for s in recent), 10) if recent else 0
            cpu, gpu = (scaled(last[key], 1000) if frame else 0 for key in ('cpu_draw_ms', 'gpu_total_ms'))
            draw = scaled(timings['samples'][frame]['tracked_image_and_work_allocation_bytes'] / 1048576, 10)
            bundle = scaled(stats['active_bundle_allocation_bytes'] / 1048576, 1000)
            clock = scaled(reference['time'], 10)
            phase = min(2, math.floor(reference['time'] / 60 * 3)) + 1
            lines = ['THEIA MIXED', f'FPS {fps / 10:.1f}', f'CPU MS {cpu / 1000:.3f}', f'GPU MS {gpu / 1000:.3f}',
                     f'MESHLETS {static_count}/{len(hulls)} TEX 4', f'BUNDLE MIB {bundle / 1000:.3f}',
                     f'DRAW MIB {draw / 10:.1f}', f'PUB {stats["publications"]} DEF {stats["deferrals"]}',
                     f'PHASE {phase} T {clock / 10:.1f}', f'PROXIES {count}/{players}', f'NEAR {near} FAR {far}']
            font = [(int(a, 16), int(b)) for a, b in re.findall(r'uint2\(0x([0-9a-f]+)u,(\d+)u\)', Path('shaders/render/benchmark_hud.slang').read_text())]
            chars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ./'
            checks = 0
            for line, text in enumerate(lines):
                for column, char in enumerate(text):
                    for row in range(7):
                        for cell in range(5):
                            i = chars.find(char)
                            bits = (((font[i][0] >> (row * 5)) & 31) if row < 6 else font[i][1]) if i >= 0 else 0
                            lit = bits & (1 << (4 - cell)) != 0
                            x = 24 + column * 18 + cell * 3 + 1
                            yy = 24 + line * 24 + row * 3 + 1
                            actual = list(rgb[(yy * width + x) * 4:(yy * width + x) * 4 + 3])
                            if color.format.BGRAOrder():
                                actual.reverse()
                            expected = list(map(encode, [.7, .92, .76] if lit else [.012, .025, .04]))
                            require(all(abs(a - b) <= 2 for a, b in zip(actual, expected)), 'Mixed stats glyph differs: ' + text)
                            checks += 1
            report.update(overlay_lines=lines, overlay_glyph_pixels=checks, overlay_verified=True)
        save = rd.TextureSave()
        save.resourceId = present.copyDestination
        save.destType = rd.FileType.PNG
        success(controller.SaveTexture(save, str(output / 'preview.png')))
        report.update(passed=True, capture=capture, players=players, frame=frame, overlay=overlay,
                      static_visible=static_ids, proxy_visible=count, near=near, far=far,
                      maximum_pose_error=maximum, world_pixels=world_pixels, proxy_pixels=proxy_pixels,
                      all_proxy_records_verified=True, both_compactions_verified=True,
                      mixed_image_identity_depth_verified=True, ray_samples=samples,
                      occlusion_probes=occlusion, boundary_envelope_probes=boundaries)
    finally:
        if controller is not None:
            controller.Shutdown()
        cap.Shutdown()


report = {'passed': False, 'renderdoc': rd.GetVersionString(), 'validation_in_capture': False}
code = 0
try:
    qualify(report)
except Exception:
    report['error'] = traceback.format_exc()
    code = 1
finally:
    output = Path(os.environ['THEIA_MIXED_CAPTURE_OUTPUT'])
    output.mkdir(parents=True, exist_ok=True)
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
os._exit(code)
