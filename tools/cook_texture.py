#!/usr/bin/env python3
"""Qualify pinned offline KTX-Software output; package bounded opaque RGBA8 sRGB.

Input schema 1 JSON contains width, height and row-major rgba integer texels.
Opt-in --png accepts bounded noninterlaced 8-bit RGBA PNG decoded by KTX offline.
Only KTX metadata/index packaging is performed here; the official tool writes DFD
and image data. No native texture decoder becomes a runtime dependency.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

SOURCE_SHA256 = '9412cb45045a503005acd47d98f9e8b47154634a50b4df21e17a1dfa8971d323'
MAX_BYTES = 4 * 1024 * 1024


def bounded(path, limit):
    with path.open('rb') as stream:
        data = stream.read(limit + 1)
    if len(data) > limit:
        raise ValueError('Texture input exceeds byte budget')
    return data


def pixels(source):
    doc = json.loads(source)
    if not isinstance(doc, dict) or type(doc.get('schema')) is not int or doc['schema'] != 1:
        raise ValueError('Texture source requires schema 1')
    width, height = doc.get('width'), doc.get('height')
    if any(type(v) is not int or not 1 <= v <= 1024 for v in (width, height)):
        raise ValueError('Texture dimensions require 1..1024')
    rgba = doc.get('rgba')
    if not isinstance(rgba, list) or len(rgba) != width * height:
        raise ValueError('Dense texture row count mismatch')
    for texel in rgba:
        if not isinstance(texel, list) or len(texel) != 4 or any(type(v) is not int or not 0 <= v <= 255 for v in texel) or texel[3] != 255:
            raise ValueError('Texture requires integer opaque RGBA8 texels')
    return width, height, bytes(v for texel in rgba for v in texel)


def package(blob, provenance):
    """Append a namespaced KTX key and relocate the sole level; preserve texels/DFD."""
    if len(blob) < 196 or blob[:12] != b'\xabKTX 20\xbb\r\n\x1a\n':
        raise ValueError('Native compiler did not emit KTX2')
    dfd, dfd_size, kvd, kvd_size = struct.unpack_from('<4I', blob, 48)
    level, size, unpacked = struct.unpack_from('<3Q', blob, 80)
    if dfd != 104 or dfd_size != 92 or kvd != 196 or level != kvd+kvd_size or size != unpacked or level+size != len(blob):
        raise ValueError('Native KTX2 layout outside qualification profile')
    entry = b'THEIAprovenance\0' + json.dumps(provenance, sort_keys=True, separators=(',', ':')).encode() + b'\0'
    added = struct.pack('<I', len(entry)) + entry + bytes((-len(entry)) % 4)
    result = bytearray(blob[:level] + added + blob[level:])
    struct.pack_into('<I', result, 60, kvd_size+len(added))
    struct.pack_into('<Q', result, 80, level+len(added))
    if len(result) > MAX_BYTES or kvd_size+len(added) > 16384:
        raise ValueError('Cooked texture exceeds encoded/metadata budget')
    return bytes(result)


def png_profile(source):
    if len(source)>MAX_BYTES or len(source)<33 or source[:16]!=b'\x89PNG\r\n\x1a\n\0\0\0\rIHDR' or source[24:29]!=bytes([8,6,0,0,0]):
        raise ValueError('PNG profile requires bounded noninterlaced 8-bit RGBA PNG')
    width,height=struct.unpack_from('>2I',source,16)
    if not 1<=width<=1024 or not 1<=height<=1024:
        raise ValueError('PNG dimensions exceed qualification profile')
    return width,height


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ktx', type=Path, required=True)
    parser.add_argument('--build-dir', type=Path, required=True)
    parser.add_argument('--source-archive', type=Path, required=True)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--png', action='store_true')
    parser.add_argument('--output-dir', type=Path, default=Path('build/assets'))
    args = parser.parse_args()
    try:
        if args.source_archive.stat().st_size > 256*1024*1024:
            raise ValueError('KTX source archive exceeds qualification budget')
        with args.source_archive.open('rb') as stream:
            archive_digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        if archive_digest != SOURCE_SHA256:
            raise ValueError('KTX source archive does not match qualified v4.4.2 digest')
        tool = args.ktx.resolve()
        build = args.build_dir.resolve()
        cache = bounded(build/'CMakeCache.txt', 1024*1024).decode()
        if tool != (build/'Release/ktx').resolve() or 'KTX_FEATURE_ETC_UNPACK:BOOL=OFF\n' not in cache or 'CMAKE_BUILD_TYPE:STRING=Release\n' not in cache:
            raise ValueError('Expected qualified Release build with ETC decoder disabled')
        commands = json.loads(bounded(build/'compile_commands.json', 16*1024*1024))
        if any(Path(command['file']).name == 'etcdec.cxx' for command in commands):
            raise ValueError('ETC decoder unexpectedly present in native compile commands')
        version = subprocess.check_output([str(tool), '--version'], text=True).strip()
        if version != 'ktx version: v4.4.2':
            raise ValueError('KTX compiler version mismatch')
        source = bounded(args.source, 64*1024*1024)
        if args.png:
            width,height=png_profile(source);rgba=None
        else:
            width, height, rgba = pixels(source)
        # These tools may be native code; all processing remains offline and bounded.
        with tempfile.TemporaryDirectory(prefix='theia-ktx-') as directory:
            raw, cooked = Path(directory)/('source.png' if args.png else 'source.raw'), Path(directory)/'texture.ktx2'
            raw.write_bytes(source if args.png else rgba)
            command=[str(tool),'create','--format','R8G8B8A8_SRGB']
            if not args.png:command+=['--raw','--width',str(width),'--height',str(height)]
            command+=['--assign-tf','srgb','--assign-primaries','bt709','--assign-texcoord-origin','top-left',str(raw),str(cooked)]
            subprocess.run(command,check=True,timeout=60,stdout=sys.stderr)
            blob = bounded(cooked, MAX_BYTES)
            # Verify raw texels before metadata packaging; no implicit conversion.
            level, size, _ = struct.unpack_from('<3Q', blob, 80)
            if struct.unpack_from('<2I',blob,20)!=(width,height) or size!=width*height*4 or (rgba is not None and blob[level:level+size]!=rgba):
                raise ValueError('Native texture pixels differ from source')
            if any(value!=255 for value in blob[level+3:level+size:4]):
                raise ValueError('PNG/native texture requires opaque decoded alpha')
            library = tool.parent/'libktx.so.4'
            if not library.is_file():
                library = tool.parent.parent/'lib/libktx.so.4'
            provenance = {'producer': 'KTX-Software-4.4.2', 'source_archive_sha256': archive_digest,
                          'tool_sha256': hashlib.sha256(tool.read_bytes()).hexdigest(),
                          'library_sha256': hashlib.sha256(library.read_bytes()).hexdigest(),
                          'source_sha256': hashlib.sha256(source).hexdigest(),
                          'cooker_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                          'compiler': subprocess.check_output(['c++', '--version'], text=True).splitlines()[0],
                          'recipe': 'Release; KTX_FEATURE_ETC_UNPACK=OFF; '+('PNG decoded offline to ' if args.png else 'raw ')+'RGBA8 sRGB/BT709; top-left; one mip; opaque'}
            blob = package(blob, provenance)
            cooked.write_bytes(blob)
            subprocess.run([str(tool), 'validate', str(cooked)], check=True, timeout=60, stdout=sys.stderr)
        digest = hashlib.sha256(blob).hexdigest()
        args.output_dir.mkdir(parents=True, exist_ok=True)
        path = args.output_dir/(digest+'.ktx2')
        path.write_bytes(blob)
        print(json.dumps({'path': str(path), 'content_id': digest, 'bytes': len(blob), 'width': width, 'height': height,
                          'texels_verified': rgba is not None, 'opaque_texels_verified': True, 'official_ktx_validation': True, 'provenance': provenance}, indent=2))
    except (OSError, ValueError, TypeError, struct.error, subprocess.SubprocessError) as error:
        parser.exit(1, f'Texture cooking failed: {error}\n')


if __name__ == '__main__':
    main()
