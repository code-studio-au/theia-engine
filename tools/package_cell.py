#!/usr/bin/env python3
"""Package a bounded, content-addressed prototype cell manifest (mesh pages only)."""
import argparse
import hashlib
from pathlib import Path
import struct


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cell', type=int, required=True)
    parser.add_argument('--output-dir', type=Path, default=Path('build/assets'))
    parser.add_argument('pages', type=Path, nargs='+')
    args = parser.parse_args()
    if not 0 <= args.cell < 2**64 or not 1 <= len(args.pages) <= 4096:
        parser.error('cell must be u64 and page count must be 1..4096')
    records = {}
    for path in args.pages:
        with path.open('rb') as source:
            data = source.read(64 * 1024 * 1024 + 1)
        if not 76 <= len(data) <= 64 * 1024 * 1024 or data[:8] != b'THEIAMES':
            parser.error(f'invalid mesh page header/size: {path}')
        digest = hashlib.sha256(data).digest()
        if path.stem != digest.hex() or digest in records:
            parser.error(f'page filename must match hash; duplicates forbidden: {path}')
        records[digest] = len(data)
    data = b'THEIACEL' + struct.pack('<IIQ', 1, len(records), args.cell)
    data += b''.join(digest + struct.pack('<Q', size) for digest, size in sorted(records.items()))
    args.output_dir.mkdir(parents=True, exist_ok=True)
    output = args.output_dir / (hashlib.sha256(data).hexdigest() + '.theia-cell')
    output.write_bytes(data)
    print(output)


if __name__ == '__main__':
    main()
