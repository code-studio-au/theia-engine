#!/usr/bin/env python3
"""Cook a bounded exact-material-bound, hash-sorted resident texture mapping."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from cook_texture import bounded


def cook(materials,source):
    if len(materials)<96 or materials[:8]!=b'THEIAMAT' or struct.unpack_from('<I',materials,8)[0]!=1 or hashlib.sha256(materials[96:]).digest()!=materials[64:96] or struct.unpack_from('<I',materials,28)[0]!=len(materials)-96:
        raise ValueError('Texture table requires THEIAMAT v1')
    count=struct.unpack_from('<I',materials,16)[0];material_id=hashlib.sha256(materials).hexdigest();doc=json.loads(source)
    if not isinstance(doc,dict) or type(doc.get('schema')) is not int or doc['schema']!=1 or doc.get('material_sha256')!=material_id or not 1<=count<=256:
        raise ValueError('Texture assignments require schema 1/exact material SHA-256')
    paths=doc.get('textures');mapping=doc.get('material_textures')
    if not isinstance(paths,list) or not 1<=len(paths)<=16 or any(not isinstance(v,str) for v in paths) or not isinstance(mapping,list) or len(mapping)!=count or any(type(v) is not int or not 0<=v<len(paths) for v in mapping):
        raise ValueError('Invalid texture dependencies/material mapping')
    if set(mapping)!=set(range(len(paths))):raise ValueError('Every texture dependency must be referenced')
    dependencies=[]
    for path in paths:
        path=Path(path);data=bounded(path,4*1024*1024);digest=hashlib.sha256(data).digest()
        if len(data)<104 or path.suffix!='.ktx2' or path.stem!=digest.hex():raise ValueError('Invalid texture filename/encoded size')
        dependencies.append((digest,len(data)))
    ordered=sorted(dependencies)
    if len({dep[0] for dep in ordered})!=len(ordered):raise ValueError('Duplicate texture dependency')
    remap={dep[0]:i for i,dep in enumerate(ordered)};indices=[remap[dependencies[i][0]] for i in mapping]
    provenance=json.dumps({'producer':'theia-texture-table-v1','source_sha256':hashlib.sha256(source).hexdigest(),
                           'cooker_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                           'sampling':'opaque RGBA8 sRGB; nearest repeat mip 0; canonical hash-sorted descriptors'},sort_keys=True,separators=(',',':')).encode()
    payload=provenance+bytes((-len(provenance))%4)+b''.join(digest+struct.pack('<Q',size) for digest,size in ordered)+struct.pack('<'+'I'*len(indices),*indices)
    return struct.pack('<8s4I32s32s',b'THEIATEX',1,len(provenance),count,len(ordered),bytes.fromhex(material_id),hashlib.sha256(payload).digest())+payload


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--materials',type=Path,required=True);parser.add_argument('--assignments',type=Path,required=True)
    parser.add_argument('--output-dir',type=Path,default=Path('build/assets'));args=parser.parse_args()
    try:
        materials=bounded(args.materials,4*1024*1024)
        if args.materials.stem!=hashlib.sha256(materials).hexdigest():raise ValueError('Material filename/content hash mismatch')
        blob=cook(materials,bounded(args.assignments,1024*1024));digest=hashlib.sha256(blob).hexdigest()
        args.output_dir.mkdir(parents=True,exist_ok=True);path=args.output_dir/(digest+'.theia-textures');path.write_bytes(blob)
        print(json.dumps({'path':str(path),'content_id':digest,'bytes':len(blob),'material_id':args.materials.stem},indent=2))
    except (OSError,ValueError,TypeError,struct.error) as error:parser.exit(1,f'Texture table cooking failed: {error}\n')


if __name__=='__main__':main()
