#!/usr/bin/env python3
"""Cook a diagnostic second-chunk palette variant through the existing offline tools.

Geometry/UV/material assignments are unchanged. The variant makes accidental
cross-chunk texture bindings visible to the replay oracle. No runtime dependency.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
from cook_benchmark_scene import png, texture


def digest(blob):
    return hashlib.sha256(blob).hexdigest()


def cook(args):
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    (out/'scene.json').write_text('{"passed":false}\n')
    scene = json.loads(args.scene.read_bytes())
    if not scene['passed'] or len(scene['quadrants']) != 4:
        raise ValueError('Needs checked four-quadrant source fixture')
    original = scene['quadrants'][2]
    source = json.loads(Path(original['source']).read_bytes())
    modified = copy.deepcopy(source)
    if len(modified['images']) != 4:
        raise ValueError('Expected four diagnostic material images')
    expected = []
    for i, image in enumerate(modified['images']):
        pixels = bytearray(texture(i % args.images))
        for pixel in range(0,len(pixels),4):
            r,g,b,_ = pixels[pixel:pixel+4]
            pixels[pixel:pixel+4] = bytes((255-b,255-r,255-g,255))
        if i < args.images:
            expected.append(bytes(pixels))
        name = 'palette-%d.png' % i
        (out/name).write_bytes(png(pixels))
        image['uri'] = name
    source_path = out/'variant.gltf'
    source_path.write_text(json.dumps(modified,sort_keys=True,separators=(',',':'))+'\n')
    normalized = out/'variant.json'
    with (out/'import.log').open('w') as log:
        subprocess.run([str(args.importer.resolve()),str(source_path),str(normalized),
                        '--unlit-textures'],stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
    a = json.loads(Path(original['normalized']).read_bytes())
    b = json.loads(normalized.read_bytes())
    for key in ('positions','indices','texcoords','triangle_materials','base_colors'):
        if a[key] != b[key]:
            raise ValueError('Palette-only variant changed '+key)
    if b['texture_materials'] != [i % args.images for i in range(4)] or len(b['textures']) != args.images:
        raise ValueError('Normalized PNG alias mapping differs from source recipe')
    command = [sys.executable,'tools/cook_meshlet_fixture.py','--geometry',str(normalized),
               '--library',str(args.library.resolve()),'--source-archive',str(args.meshoptimizer_source.resolve()),
               '--ktx',str(args.ktx.resolve()),'--ktx-build',str(args.ktx_build.resolve()),
               '--ktx-source-archive',str(args.ktx_source.resolve()),'--output-dir',str(out/'cooked')]
    with (out/'cook.log').open('w') as log:
        subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=240)
    cooked = json.loads((out/'cooked/imported-report.json').read_bytes())
    if not cooked['coverage_and_winding_verified'] or len(cooked['textures']) != args.images:
        raise ValueError('Variant geometry/texture proof missing')
    for record, pixels in zip(cooked['textures'],expected):
        encoded = Path(record['path']).read_bytes()
        offset,size = struct.unpack_from('<2Q',encoded,80)
        if (not record['official_ktx_validation']
                or encoded[offset:offset+size] != pixels):
            raise ValueError('Native KTX output differs from exact variant pixels')
    variant = copy.deepcopy(scene)
    variant['quadrants'][2].update(source=str(source_path),source_sha256=digest(source_path.read_bytes()),
        normalized=str(normalized),normalized_sha256=digest(normalized.read_bytes()),cook=cooked)
    variant['palette_variant'] = dict(chunk=2,recipe='rgb=(255-b,255-r,255-g),alpha=255',
        source_scene_sha256=digest(args.scene.read_bytes()), geometry_uv_materials_unchanged=True, unique_images=args.images, texture_aliases_verified=True,
        native_texels_verified=True)
    (out/'scene.json').write_text(json.dumps(variant,indent=2,sort_keys=True)+'\n')
    print('Cooked palette-only chunk variant; unchanged geometry/UVs/material assignments and exact native texels')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--images',type=int,choices=(2,4),default=4)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--importer',type=Path,default=Path('tools/theia-gltf-import/target/debug/theia-gltf-import'))
    parser.add_argument('--library',type=Path,default=Path('build/qualification/meshoptimizer/lib-build/libmeshoptimizer.so'))
    parser.add_argument('--meshoptimizer-source',type=Path,default=Path('build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz'))
    parser.add_argument('--ktx',type=Path,default=Path('build/qualification/ktx-tool/source-build/Release/ktx'))
    parser.add_argument('--ktx-build',type=Path,default=Path('build/qualification/ktx-tool/source-build'))
    parser.add_argument('--ktx-source',type=Path,default=Path('build/qualification/ktx-tool/ktx-source-v4.4.2.tar.gz'))
    cook(parser.parse_args())
