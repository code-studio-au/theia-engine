#!/usr/bin/env python3
"""Cook bounded opaque unlit factors and explicit cooked-triangle assignments.

Input assignments bind the exact mesh SHA-256; they are not glTF primitive indices.
Rust MaterialPage validates the emitted sidecar against fully decoded geometry.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct


def bounded(path, limit):
    with path.open('rb') as stream:
        result = stream.read(limit + 1)
    if len(result) > limit:
        raise ValueError('Input exceeds byte budget')
    return result


def cook(mesh, source):
    identifier = hashlib.sha256(mesh).hexdigest()
    if len(mesh) < 76 or mesh[:8] != b'THEIAMES':
        raise ValueError('Invalid geometry header')
    version, meta, vertices, count, refs, triangles, max_v, max_t, payload = struct.unpack_from('<9I', mesh, 8)
    if version != 1 or not 1 <= count <= 4096 or not 1 <= max_v <= 64 or not 1 <= max_t <= 64:
        raise ValueError('Unsupported geometry version/counts')
    if payload != len(mesh)-76 or hashlib.sha256(mesh[76:]).digest() != mesh[44:76]:
        raise ValueError('Geometry payload length/digest mismatch')
    expected = ((meta+3)&~3) + vertices*12 + count*16 + refs*4 + ((triangles+3)&~3)
    if expected != payload:
        raise ValueError('Geometry layout mismatch')
    document = json.loads(source)
    if not isinstance(document,dict) or type(document.get('schema')) is not int or document['schema'] != 1 or document.get('geometry_sha256') != identifier:
        raise ValueError('Assignments require schema 1 and exact geometry content address')
    colors = document.get('base_colors')
    assignments = document.get('triangle_materials')
    if not isinstance(colors, list) or not 1 <= len(colors) <= 256:
        raise ValueError('Material count must be 1..256')
    for color in colors:
        if not isinstance(color, list) or len(color) != 4 or any(type(v) not in (int,float) or not math.isfinite(v) or not 0 <= v <= 1 for v in color) or color[3] != 1:
            raise ValueError('Factors must be finite opaque linear RGBA')
    offsets = [0]
    meshlets_offset = 76 + ((meta+3)&~3) + vertices*12
    for i in range(count):
        vertex_offset, triangle_offset, vertex_count, triangle_count = struct.unpack_from('<4I', mesh, meshlets_offset+i*16)
        if not 1 <= vertex_count <= max_v or not 1 <= triangle_count <= max_t or vertex_offset+vertex_count > refs or triangle_offset+triangle_count*3 > triangles:
            raise ValueError('Invalid meshlet ranges')
        offsets.append(offsets[-1]+triangle_count)
    if not isinstance(assignments,list) or len(assignments) != offsets[-1] or any(type(v) is not int or not 0 <= v < len(colors) for v in assignments):
        raise ValueError('Explicit triangle assignment count/references mismatch')
    provenance = json.dumps({'producer':'theia-unlit-material-v1','source_sha256':hashlib.sha256(source).hexdigest(),
                             'cooker_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                             'shading':'opaque unlit; linear base-color factor; no textures or vertex attributes'},sort_keys=True,separators=(',',':')).encode()
    data = provenance + bytes((-len(provenance))%4)
    data += b''.join(struct.pack('<4f',*color) for color in colors)
    data += struct.pack(f'<{len(offsets)}I',*offsets)
    data += struct.pack(f'<{len(assignments)}I',*assignments)
    header = struct.pack('<8s6I32s32s',b'THEIAMAT',1,len(provenance),len(colors),count,len(assignments),len(data),bytes.fromhex(identifier),hashlib.sha256(data).digest())
    return header+data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mesh',type=Path,required=True)
    parser.add_argument('--assignments',type=Path,required=True)
    parser.add_argument('--output-dir',type=Path,default=Path('build/assets'))
    args = parser.parse_args()
    try:
        mesh = bounded(args.mesh,64*1024*1024)
        if args.mesh.stem != hashlib.sha256(mesh).hexdigest():
            raise ValueError('Mesh filename must match content address')
        blob = cook(mesh,bounded(args.assignments,4*1024*1024))
        identifier = hashlib.sha256(blob).hexdigest()
        args.output_dir.mkdir(parents=True,exist_ok=True)
        destination = args.output_dir/(identifier+'.theia-material')
        destination.write_bytes(blob)
        print(json.dumps({'path':str(destination),'content_id':identifier,'bytes':len(blob),'geometry_id':args.mesh.stem},indent=2))
    except (OSError,ValueError,TypeError,struct.error) as error:
        parser.exit(1,f'Material cooking failed: {error}\n')


if __name__ == '__main__':
    main()
