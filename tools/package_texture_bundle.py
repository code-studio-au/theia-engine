#!/usr/bin/env python3
"""Package four content-addressed dependencies; Rust admission validates semantics."""
import argparse
import hashlib
import json
from pathlib import Path
import struct


def package(paths,indexed=False):
    suffixes=['.theia-mesh','.theia-material','.theia-uv','.theia-textures' if indexed else '.ktx2']
    minima=[76,96,88,88 if indexed else 104];budgets=[64*1024*1024,4*1024*1024,4*1024*1024,(1 if indexed else 4)*1024*1024]
    data=b'THEIABND'+struct.pack('<2I',2 if indexed else 1,4)
    ids=set()
    for path,suffix,minimum,budget in zip(paths,suffixes,minima,budgets,strict=True):
        with path.open('rb') as stream:child=stream.read(budget+1)
        digest=hashlib.sha256(child).hexdigest()
        if path.suffix!=suffix or path.stem!=digest or not minimum<=len(child)<=budget or digest in ids:
            raise ValueError('Invalid bundle child filename/role/size/duplicate')
        ids.add(digest);data+=bytes.fromhex(digest)+struct.pack('<Q',len(child))
    return data


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for role in ('mesh','materials','uvs'):parser.add_argument('--'+role,type=Path,required=True)
    group=parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--texture',type=Path);group.add_argument('--texture-table',type=Path)
    parser.add_argument('--output-dir',type=Path,default=Path('build/assets'))
    args=parser.parse_args()
    try:
        paths=[args.mesh,args.materials,args.uvs,args.texture or args.texture_table]
        # Children must already be colocated; no implicit copying/publication.
        if any(path.parent.resolve()!=args.output_dir.resolve() for path in paths):raise ValueError('Bundle children must share output directory')
        blob=package(paths,args.texture_table is not None);digest=hashlib.sha256(blob).hexdigest();args.output_dir.mkdir(parents=True,exist_ok=True)
        path=args.output_dir/(digest+'.theia-bundle');path.write_bytes(blob)
        print(json.dumps({'path':str(path),'content_id':digest,'bytes':len(blob),'roles':['geometry','materials','uvs','texture-table' if args.texture_table else 'texture']},indent=2))
    except (OSError,ValueError) as error:parser.exit(1,f'Texture bundle packaging failed: {error}\n')


if __name__=='__main__':main()
