#!/usr/bin/env python3
"""Cook a complete authored opaque RGBA8 sRGB mip chain with pinned offline KTX.

Schema 1 source: levels, largest first, each containing width/height/rgba.
No filtering or implicit mip generation: every supplied texel must survive exactly.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
from cook_texture import MAX_BYTES, bounded, pixels


def source_levels(source):
    doc = json.loads(source)
    if not isinstance(doc, dict) or type(doc.get('schema')) is not int or doc['schema'] != 1:
        raise ValueError('Mip source requires schema 1')
    levels = doc.get('levels')
    if not isinstance(levels, list) or not 1 <= len(levels) <= 11:
        raise ValueError('Mip source requires 1..11 explicit levels')
    result = []
    for index, level in enumerate(levels):
        if not isinstance(level, dict):
            raise ValueError('Invalid authored mip object')
        width, height, rgba = pixels(json.dumps({**level, 'schema': 1}))
        if index == 0:
            base = (width, height)
            if len(levels) != max(base).bit_length():
                raise ValueError('Mip source must reach the complete 1x1 tail')
        if (width, height) != tuple(max(1, v >> index) for v in base):
            raise ValueError('Authored mip dimensions do not match the base extent')
        result.append((width, height, rgba))
    if sum(len(v[2]) for v in result) > MAX_BYTES:
        raise ValueError('Mip source texels exceed byte budget')
    return result


def package_mips(blob, provenance):
    if len(blob) < 104 or blob[:12] != b'\xabKTX 20\xbb\r\n\x1a\n':
        raise ValueError('Native compiler did not emit KTX2')
    width, height = struct.unpack_from('<2I', blob, 20)
    count = struct.unpack_from('<I', blob, 40)[0]
    if not 1 <= width <= 1024 or not 1 <= height <= 1024 or count != max(width, height).bit_length():
        raise ValueError('Native compiler did not emit a complete bounded mip chain')
    dfd, dfd_size, kvd, kvd_size = struct.unpack_from('<4I', blob, 48)
    start = kvd + kvd_size
    if dfd != 80 + count * 24 or dfd_size != 92 or kvd != dfd + 92 or not 0 < kvd_size <= 16384 or kvd_size % 4 or start > len(blob):
        raise ValueError('Native mip sections outside qualification profile')
    cursor = start
    for index in reversed(range(count)):
        offset, size, unpacked = struct.unpack_from('<3Q', blob, 80 + index * 24)
        expected = max(1, width >> index) * max(1, height >> index) * 4
        if offset != cursor or size != expected or unpacked != expected:
            raise ValueError('Native mip layout outside qualification profile')
        cursor += size
    if cursor != len(blob):
        raise ValueError('Native mip payload length mismatch')
    entry = b'THEIAprovenance\0' + json.dumps(provenance, sort_keys=True, separators=(',', ':')).encode() + b'\0'
    added = struct.pack('<I', len(entry)) + entry + bytes((-len(entry)) % 4)
    result = bytearray(blob[:start] + added + blob[start:])
    struct.pack_into('<I', result, 60, kvd_size + len(added))
    for index in range(count):
        position = 80 + index * 24
        offset = struct.unpack_from('<Q', blob, position)[0]
        struct.pack_into('<Q', result, position, offset + len(added))
    if len(result) > MAX_BYTES or kvd_size + len(added) > 16384:
        raise ValueError('Cooked mip chain exceeds encoded/metadata budget')
    return bytes(result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['ktx', 'build-dir', 'source-archive', 'source', 'output-dir']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    try:
        source = bounded(args.source, 64 * 1024 * 1024)
        levels = source_levels(source)
        with tempfile.TemporaryDirectory(prefix='theia-mips-') as directory:
            directory = Path(directory)
            gate_source = directory / 'base.json'
            gate_source.write_text(json.dumps({'schema': 1, 'width': levels[0][0],
                'height': levels[0][1], 'rgba': json.loads(source)['levels'][0]['rgba']}))
            # Reuse the existing strict source-build/archive/license/version gate.
            # Its single-level output stays temporary; only the full chain publishes.
            gate = json.loads(subprocess.check_output([sys.executable,
                str(Path(__file__).with_name('cook_texture.py')), '--ktx', str(args.ktx),
                '--build-dir', str(args.build_dir), '--source-archive', str(args.source_archive),
                '--source', str(gate_source), '--output-dir', str(directory)], timeout=120))
            raw = []
            for index, (_, _, rgba) in enumerate(levels):
                path = directory / f'level-{index}.raw'
                path.write_bytes(rgba)
                raw.append(str(path))
            output = directory / 'mips.ktx2'
            tool = str(args.ktx.resolve())
            subprocess.run([tool, 'create', '--raw', '--format', 'R8G8B8A8_SRGB',
                '--width', str(levels[0][0]), '--height', str(levels[0][1]),
                '--levels', str(len(levels)), '--assign-tf', 'srgb', '--assign-primaries',
                'bt709', '--assign-texcoord-origin', 'top-left', *raw, str(output)],
                check=True, timeout=60, stdout=sys.stderr)
            blob = bounded(output, MAX_BYTES)
            for index, (_, _, rgba) in enumerate(levels):
                offset, size, unpacked = struct.unpack_from('<3Q', blob, 80 + index * 24)
                if size != len(rgba) or unpacked != size or blob[offset:offset + size] != rgba:
                    raise ValueError('Native authored mip texels differ from source')
            provenance = dict(gate['provenance'], source_sha256=hashlib.sha256(source).hexdigest(),
                cooker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                recipe='Release; KTX_FEATURE_ETC_UNPACK=OFF; complete authored raw RGBA8 sRGB/BT709 mips; top-left; opaque; no filtering')
            blob = package_mips(blob, provenance)
            output.write_bytes(blob)
            subprocess.run([tool, 'validate', str(output)], check=True, timeout=60, stdout=sys.stderr)
        digest = hashlib.sha256(blob).hexdigest()
        args.output_dir.mkdir(parents=True, exist_ok=True)
        path = args.output_dir / (digest + '.ktx2')
        path.write_bytes(blob)
        print(json.dumps({'path': str(path), 'content_id': digest, 'bytes': len(blob),
            'levels': [{'width': w, 'height': h, 'texel_bytes': len(rgba)} for w, h, rgba in levels],
            'texels_verified': True, 'official_ktx_validation': True, 'provenance': provenance}, indent=2))
    except (OSError, ValueError, TypeError, struct.error, subprocess.SubprocessError) as error:
        parser.exit(1, f'Mip cooking failed: {error}\n')


if __name__ == '__main__':
    main()
