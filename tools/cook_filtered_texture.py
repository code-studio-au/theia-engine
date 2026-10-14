#!/usr/bin/env python3
"""Qualify native KTX box-generated mip chains for opaque power-of-two sources.

Schema-1 raw RGBA8 JSON is losslessly wrapped as PNG for the native generator
(KTX 4.4.2 does not generate mips from --raw). No Python production filtering.
Independent f64 box checks validate native lower levels before publication.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import zlib
from cook_texture import MAX_BYTES, bounded, pixels
from cook_texture_mips import package_mips


def filtered_source(source):
    width, height, rgba = pixels(source)
    if any(v & (v - 1) for v in [width, height]):
        raise ValueError('Filtered source requires power-of-two dimensions')
    count = max(width, height).bit_length()
    if sum(max(1, width >> i) * max(1, height >> i) * 4 for i in range(count)) > MAX_BYTES:
        raise ValueError('Generated chain dense bytes exceed encoded budget')
    return width, height, rgba


def png_envelope(width, height, rgba):
    def chunk(tag, data):
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data))
    rows = b''.join(b'\0' + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>2I5B', width, height, 8, 6, 0, 0, 0))
            + chunk(b'sRGB', b'\0') + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b''))


def decode(v):
    v /= 255
    return v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4


def encode(v):
    return math.floor((v * 12.92 if v <= .0031308 else 1.055 * v ** (1 / 2.4) - .055) * 255 + .5)


def verify_generated(blob, width, height, rgba):
    count = max(width, height).bit_length()
    if len(blob) < 80 + count * 24 or blob[:12] != b'\xabKTX 20\xbb\r\n\x1a\n' or struct.unpack_from('<I', blob, 12)[0] != 43:
        raise ValueError('Invalid generated RGBA8 sRGB KTX header')
    if struct.unpack_from('<2I', blob, 20) != (width, height) or struct.unpack_from('<I', blob, 40)[0] != count:
        raise ValueError('Native generator changed extent or complete level count')
    levels = []
    previous = None
    for index in range(count):
        w, h = max(1, width >> index), max(1, height >> index)
        offset, size, unpacked = struct.unpack_from('<3Q', blob, 80 + index * 24)
        if size != w * h * 4 or unpacked != size or offset < 80 + count * 24 or offset + size > len(blob):
            raise ValueError('Invalid generated level byte range')
        texels = blob[offset:offset + size]
        if any(v != 255 for v in texels[3::4]):
            raise ValueError('Generated mip alpha is not opaque')
        error = 0
        if index == 0:
            if texels != rgba:
                raise ValueError('Native generator changed base source texels')
        else:
            pw, ph, raw = previous
            for y in range(h):
                for x in range(w):
                    taps = [((y * (2 if ph > 1 else 1) + dy) * pw
                             + x * (2 if pw > 1 else 1) + dx) * 4
                            for dy in range(2 if ph > 1 else 1) for dx in range(2 if pw > 1 else 1)]
                    for c in range(3):
                        expected = encode(math.fsum(decode(raw[t + c]) for t in taps) / len(taps))
                        error = max(error, abs(texels[(y * w + x) * 4 + c] - expected))
            if error > 1:
                raise ValueError('Native generated mip differs from independent linear-sRGB box reference')
        levels.append({'width': w, 'height': h, 'texel_bytes': size, 'max_rgb_error': error})
        previous = (w, h, texels)
    return levels


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['ktx', 'build-dir', 'source-archive', 'source', 'output-dir']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    try:
        source = bounded(args.source, 64 * 1024 * 1024)
        width, height, rgba = filtered_source(source)
        with tempfile.TemporaryDirectory(prefix='theia-filtered-mips-') as directory:
            directory = Path(directory)
            gate = json.loads(subprocess.check_output([sys.executable,
                str(Path(__file__).with_name('cook_texture.py')), '--ktx', str(args.ktx),
                '--build-dir', str(args.build_dir), '--source-archive', str(args.source_archive),
                '--source', str(args.source), '--output-dir', str(directory)], timeout=120))
            image = directory / 'input.png'
            png = png_envelope(width, height, rgba)
            image.write_bytes(png)
            output = directory / 'generated.ktx2'
            tool = str(args.ktx.resolve())
            subprocess.run([tool, 'create', '--format', 'R8G8B8A8_SRGB', '--generate-mipmap',
                '--mipmap-filter', 'box', '--mipmap-filter-scale', '1', '--mipmap-wrap', 'clamp',
                '--assign-tf', 'srgb', '--assign-primaries', 'bt709', '--assign-texcoord-origin',
                'top-left', str(image), str(output)], check=True, timeout=60, stdout=sys.stderr)
            blob = bounded(output, MAX_BYTES)
            levels = verify_generated(blob, width, height, rgba)
            provenance = dict(gate['provenance'], source_sha256=hashlib.sha256(source).hexdigest(),
                input_png_sha256=hashlib.sha256(png).hexdigest(),
                cooker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                recipe='Release; KTX_FEATURE_ETC_UNPACK=OFF; PNG envelope; opaque RGBA8 sRGB/BT709; top-left; native complete iterative box/clamp mips; independent per-step RGB tolerance 1 byte')
            blob = package_mips(blob, provenance)
            output.write_bytes(blob)
            subprocess.run([tool, 'validate', str(output)], check=True, timeout=60, stdout=sys.stderr)
        digest = hashlib.sha256(blob).hexdigest()
        args.output_dir.mkdir(parents=True, exist_ok=True)
        path = args.output_dir / (digest + '.ktx2')
        path.write_bytes(blob)
        print(json.dumps({'path': str(path), 'content_id': digest, 'bytes': len(blob), 'levels': levels,
            'texels_verified': True, 'base_texels_verified': True, 'filtered_mips_verified': True,
            'official_ktx_validation': True, 'provenance': provenance}, indent=2))
    except (OSError, ValueError, TypeError, struct.error, subprocess.SubprocessError) as error:
        parser.exit(1, f'Filtered mip cooking failed: {error}\n')


if __name__ == '__main__':
    main()
