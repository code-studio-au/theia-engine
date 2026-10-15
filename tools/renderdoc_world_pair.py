"""Owned replay oracle for two resident geometry chunks sharing identity/depth."""
import json
import math
import os
from pathlib import Path
import shlex
import struct
import subprocess
import sys
sys.dont_write_bytecode = True
import time
import traceback
import renderdoc as rd
sys.path.insert(0, str(Path.cwd() / 'tools'))
from cooked_benchmark_reference import decode, ray, dot
from proxy_benchmark_reference import encode


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
    output = Path(os.environ['THEIA_WORLD_PAIR_OUTPUT']).resolve()
    output.mkdir(parents=True, exist_ok=True)
    scene = json.loads(Path(os.environ['THEIA_WORLD_PAIR_SCENE']).read_bytes())
    indices = json.loads(os.environ.get('THEIA_WORLD_PAIR_CHUNKS', '[0,2]'))
    require(len(indices) == 2 and all(i in range(4) for i in indices), 'Invalid pair')
    cooks = [scene['quadrants'][i]['cook'] for i in indices]
    models = [decode(cook) for cook in cooks]
    view = int(os.environ.get('THEIA_WORLD_PAIR_VIEW', '2'))
    exe = Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
    args = ['--world-pair-test', *[c['path'] for c in cooks], '--frames', '90',
            '--work-items', str(view), '--no-validation', '--gpu', '7900']
    if os.environ.get('THEIA_WORLD_PAIR_SHADERS'):
        args += ['--shader-dir', os.environ['THEIA_WORLD_PAIR_SHADERS']]
    options = rd.CaptureOptions()
    options.apiValidation = False
    options.debugOutputMute = False
    launched = rd.ExecuteAndInject(str(exe), str(Path.cwd()), shlex.join(args), [],
                                   str(output / 'world'), options, False)
    success(launched.result)
    target = rd.CreateTargetControl('', launched.ident, 'Theia world pair qualification', True)
    require(target is not None, 'Missing capture target')
    capture = None
    try:
        target.QueueCapture(10, 1)
        deadline = time.monotonic() + 60
        while target.Connected() and time.monotonic() < deadline:
            message = target.ReceiveMessage(None)
            if message.type == rd.TargetControlMessageType.NewCapture:
                capture = message.newCapture.path
        require(not target.Connected() and capture, 'Missing capture or target timeout')
    finally:
        target.Shutdown()
    cap = rd.OpenCaptureFile()
    controller = None
    try:
        success(cap.OpenFile(capture, '', None))
        result, controller = cap.OpenCapture(rd.ReplayOptions(), None)
        success(result)
        actions = list(walk(controller.GetRootActions()))
        present = next(a for a in actions if a.flags & rd.ActionFlags.Present)
        controller.SetFrameEvent(present.eventId, True)
        textures = controller.GetTextures()
        color = next(t for t in textures if t.resourceId == present.copyDestination)
        width, height = color.width, color.height
        rows = json.loads(subprocess.check_output([str(exe), '--world-pair-reference',
               str(width), str(height), str(view)]))['rows']
        buffers = controller.GetBuffers()
        for model in models:
            blob = model[0][0]
            require(any(bytes(controller.GetBufferData(b.resourceId, 0, len(blob))) == blob
                        for b in buffers if b.length == len(blob)), 'GPU geometry differs')
        works = [controller.GetBufferData(b.resourceId, 0, 256) for b in buffers if b.length == 256]
        require(len(works) == 2, 'Expected two independent work arenas')
        expected_compactions = []
        visible_sets = []
        for _, triangles, hulls, _ in models:
            visible = []
            for i, hull in enumerate(hulls):
                common = 63
                for point in hull:
                    x, y, z, w = [dot(row[:3], point) + row[3] for row in rows]
                    common &= int(x < -w) | (int(x > w) << 1) | (int(y < -w) << 2) | (int(y > w) << 3) | (int(z < 0) << 4) | (int(z > w) << 5)
                if common == 0:
                    visible.append(i)
            visible_sets.append(set(visible))
            expected_compactions.append((len(visible), sum(1 << i for i in visible), visible))
        actual_compactions = []
        for work in works:
            count, mask = struct.unpack_from('<2I', work, 36)
            ids = sorted(struct.unpack_from('<%dI' % count, work, 64))
            actual_compactions.append((count, mask, ids))
        require(sorted(actual_compactions) == sorted(expected_compactions), 'World compaction differs')
        identity = next(t for t in textures if t.format.compType == rd.CompType.UInt and t.format.compCount == 2)
        depth = next(t for t in textures if t.format.compType == rd.CompType.Depth)
        ids = controller.GetTextureData(identity.resourceId, rd.Subresource())
        depths = controller.GetTextureData(depth.resourceId, rd.Subresource())
        rgb = controller.GetTextureData(color.resourceId, rd.Subresource())
        limits = [{i: sum(t[0] == i for t in m[1]) for i in range(1, len(m[2]) + 1)} for m in models]
        populations = [0, 0]
        for pixel in range(width * height):
            ident, primitive = struct.unpack_from('<2I', ids, pixel * 8)
            d = struct.unpack_from('<f', depths, pixel * 4)[0]
            require(math.isfinite(d) and 0 <= d <= 1, 'Invalid depth')
            if ident == 0:
                require(d == 1, 'Background depth changed')
            else:
                chunk, local = ident >> 30, ident & 0x3fffffff
                require(chunk < 2 and local > 0 and local - 1 in visible_sets[chunk]
                        and primitive < limits[chunk][local] and d < 1, 'Invalid world identity/primitive')
                populations[chunk] += 1
        require(all(populations) if view else not any(populations), 'Missing expected chunk coverage')
        samples = []
        occlusion = [0, 0]
        probes = {(int(width*x/10), int(height*y/8)) for x in range(1,10) for y in range(1,8)}
        for model in models:
            for tri in model[1][::max(1, len(model[1])//32)]:
                center = [sum(p[a] for p in tri[2])/3 for a in range(3)]
                clip = [dot(row[:3], center) + row[3] for row in rows]
                if clip[3] > 0:
                    x,y = [int((clip[a]/clip[3]+1)*.5*s) for a,s in enumerate((width,height))]
                    if 0 <= x < width and 0 <= y < height:
                        probes.add((x,y))
        for x,y in sorted(probes):
            hits = [ray(rows, model[1], x,y,width,height) for model in models]
            chunk = min(range(2), key=lambda i:hits[i][2])
            ident, primitive, d, _, _ = hits[chunk]
            if all(h[0] for h in hits):
                occlusion[chunk] += 1
            if ident:
                ident |= chunk << 30
                tint = [.18,.48,.24] if chunk == 0 else [.48,.24,.18]
                expected_rgb = list(map(encode, [v*(.7 if primitive%2 else 1) for v in tint]))
            else:
                expected_rgb = list(map(encode, [.025,.07,.12]))
            pixel = y*width+x
            actual = struct.unpack_from('<2I', ids, pixel*8)
            actual_depth = struct.unpack_from('<f', depths, pixel*4)[0]
            require(actual == (ident,primitive) and abs(actual_depth-d) <= 2e-5,
                    'World ray/raster differs at %d,%d: %s vs %s' % (x,y,actual,(ident,primitive)))
            actual_rgb = list(rgb[pixel*4:pixel*4+3])
            if color.format.BGRAOrder():
                actual_rgb.reverse()
            require(all(abs(a-b) <= 2 for a,b in zip(actual_rgb,expected_rgb)), 'World colour differs')
            samples.append(dict(pixel=[x,y], identity=actual, depth=actual_depth, color=actual_rgb))
        save = rd.TextureSave()
        save.resourceId, save.destType = color.resourceId, rd.FileType.PNG
        success(controller.SaveTexture(save, str(output / 'preview.png')))
        report.update(passed=True, capture=capture, chunks=indices, view=view,
                      shader_dir=os.environ.get('THEIA_WORLD_PAIR_SHADERS','build/shaders'),
                      pixels=populations, compactions_verified=True,
                      identity_depth_bounds_verified=True, ray_samples=samples,
                      occlusion_probes=occlusion)
    finally:
        if controller is not None:
            controller.Shutdown()
        cap.Shutdown()


report = dict(passed=False, renderdoc=rd.GetVersionString(), validation_in_capture=False)
code = 0
try:
    qualify(report)
except Exception:
    report['error'] = traceback.format_exc()
    code = 1
finally:
    out = Path(os.environ['THEIA_WORLD_PAIR_OUTPUT'])
    out.mkdir(parents=True, exist_ok=True)
    (out / 'report.json').write_text(json.dumps(report,indent=2)+'\n')
os._exit(code)
