#!/usr/bin/env python3
"""Build bounded test-only LOD guard fixtures from already cooked native topology.

Position rewrites and proof checks occur offline; no native simplification error
is claimed for rewritten geometry. Runtime still admits both encoded containers.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
from continuous_height_error import continuous_error
from qualify_height_lod import topology


def decode(path):
    data = path.read_bytes()
    if len(data) > 64*1024*1024 or len(data) < 76 or data[:8] != b'THEIAMES':
        raise ValueError('Invalid bounded cooked guard source')
    if hashlib.sha256(data).hexdigest() != path.stem or struct.unpack_from('<I', data, 8)[0] != 1:
        raise ValueError('Guard source version/hash mismatch')
    meta, vertices, meshlets, refs = struct.unpack_from('<4I', data, 12)
    if not 3 <= vertices <= 2048 or not 1 <= meshlets <= 32 or meta > 1024*1024:
        raise ValueError('Guard source count/metadata budget')
    if hashlib.sha256(data[76:]).digest() != data[44:76]:
        raise ValueError('Guard source payload digest mismatch')
    positions_offset = 76+(meta+3)//4*4
    meshlet_offset = positions_offset+vertices*12
    refs_offset = meshlet_offset+meshlets*16
    triangles_offset = refs_offset+refs*4
    positions = [struct.unpack_from('<3f', data, positions_offset+i*12) for i in range(vertices)]
    indices = []
    for i in range(meshlets):
        vo, to, vc, tc = struct.unpack_from('<4I', data, meshlet_offset+i*16)
        if vc > 64 or tc > 64 or vo+vc > refs:
            raise ValueError('Guard source meshlet range invalid')
        for j in range(tc*3):
            local = data[triangles_offset+to+j]
            if local >= vc:
                raise ValueError('Guard source local index invalid')
            vertex = struct.unpack_from('<I', data, refs_offset+(vo+local)*4)[0]
            if vertex >= vertices:
                raise ValueError('Guard source global index invalid')
            indices.append(vertex)
    return data, positions, indices, positions_offset, json.loads(data[76:76+meta])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fine', type=Path, required=True)
    parser.add_argument('--coarse', type=Path, required=True)
    parser.add_argument('--translation-y', type=float, default=0.)
    parser.add_argument('--small-flat-bound', action='store_true')
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    if not math.isfinite(args.translation_y) or abs(args.translation_y) > 64:
        raise ValueError('Guard translation is outside bounded test profile')
    fine, coarse = decode(args.fine), decode(args.coarse)
    if fine[1] != coarse[1]:
        raise ValueError('Guard source positions differ')
    positions = [struct.unpack('<3f', struct.pack('<3f', x, y+args.translation_y, z))
                 for x, y, z in fine[1]]
    if topology(positions,fine[2])[0]!=topology(positions,coarse[2])[0]:
        raise ValueError('Guard source boundary edges differ')
    proof = continuous_error(positions, fine[2], coarse[2], .03)
    proof.update(original_triangles=len(fine[2])//3, simplified_triangles=len(coarse[2])//3,
                 exact_boundary_preserved=True)
    if args.small_flat_bound:
        if proof['continuous_vertical_error_numerator'] != '0':
            raise ValueError('Small conservative bound override requires true zero surface error')
        proof['continuous_vertical_error_upper_bound'] = 1e-8
    position_bytes = b''.join(struct.pack('<3f', *p) for p in positions)
    source_id = hashlib.sha256(position_bytes+struct.pack(f'<{len(fine[2])}I', *fine[2])).hexdigest()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    report = {'translation_y': args.translation_y, 'small_flat_bound': args.small_flat_bound,
              'continuous_qualification': proof}
    for role, original in [('fine', fine), ('coarse', coarse)]:
        data, _, indices, offset, metadata = original
        rewritten = bytearray(data[offset:])
        rewritten[:len(position_bytes)] = position_bytes
        provenance = {'producer': 'theia-lod-guard-fixture-v1',
                      'recipe': 'test-only represented position rewrite; original native topology retained',
                      'source_sha256': source_id if role == 'fine' else hashlib.sha256(position_bytes+struct.pack(f'<{len(indices)}I', *indices)).hexdigest(),
                      'original_producer': metadata,
                      'qualification_transform': {'source_mesh_sha256': hashlib.sha256(data).hexdigest(),
                          'tool_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                          'translation_y': args.translation_y, 'small_flat_bound': args.small_flat_bound}}
        if role == 'coarse':
            provenance.update(fallback_for_source_sha256=source_id, lod_qualification=proof)
        meta = json.dumps(provenance, sort_keys=True, separators=(',', ':')).encode()
        if len(meta) > 1024*1024:
            raise ValueError('Guard rewritten metadata exceeds budget')
        payload = meta+bytes((-len(meta)) % 4)+rewritten
        header = bytearray(data[:76])
        struct.pack_into('<I', header, 12, len(meta))
        struct.pack_into('<I', header, 40, len(payload))
        header[44:76] = hashlib.sha256(payload).digest()
        encoded = header+payload
        path = args.output_dir/(hashlib.sha256(encoded).hexdigest()+'.theia-mesh')
        if path.exists() and path.read_bytes() != encoded:
            raise ValueError('Guard output content collision')
        path.write_bytes(encoded)
        report[role] = str(path)
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
