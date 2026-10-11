#!/usr/bin/env python3
"""Offline meshoptimizer 1.3 qualification: cook a synthetic grid into THEIAMES v1.

The native third-party library is only loaded here, never by shipped Rust code.
"""
import argparse
from collections import Counter
import ctypes as c
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
from qualify_height_lod import qualify
from remap_materials import canonical, remap
from cook_material_page import cook as cook_material
from cook_uv_page import cook as cook_uv


class Meshlet(c.Structure):
    _fields_ = [('vertex_offset', c.c_uint), ('triangle_offset', c.c_uint),
                ('vertex_count', c.c_uint), ('triangle_count', c.c_uint)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--library', type=Path, required=True)
    parser.add_argument('--source-archive', type=Path, required=True)
    parser.add_argument('--fixture', choices=['grid16','triangle','quad','terrain16','occlusion'], default='grid16')
    parser.add_argument('--geometry', type=Path, help='Position/index JSON from the bounded static glTF normalizer')
    parser.add_argument('--simplify-ratio', type=float, help='Bounded heightfield LOD target triangle ratio')
    parser.add_argument('--simplify-error', type=float, default=.02, help='Native absolute position error limit')
    parser.add_argument('--max-height-error', type=float, default=.03, help='Independent sampled vertical error limit')
    parser.add_argument('--output-dir', type=Path, default=Path('build/assets'))
    args = parser.parse_args()
    if args.geometry and (args.fixture != 'grid16' or args.simplify_ratio is not None):
        parser.error('Normalized geometry cannot combine with fixture selection/heightfield simplification')
    if args.simplify_ratio is not None and (args.fixture not in ('grid16','terrain16') or not 0 < args.simplify_ratio < 1):
        parser.error('Simplification requires grid16/terrain16 and ratio strictly between 0 and 1')
    if any(not math.isfinite(value) or value < 0 for value in (args.simplify_error,args.max_height_error)):
        parser.error('Error limits must be finite and nonnegative')
    archive_digest = hashlib.sha256(args.source_archive.read_bytes()).hexdigest()
    expected = 'd50a0b5e1323ba4379cdb32f89436c20652b7393445f8e18c88ae64445b67b77'
    if archive_digest != expected:
        parser.error('Source archive differs from qualified official meshoptimizer 1.3 release')
    library = c.CDLL(str(args.library.resolve()))
    library.meshopt_buildMeshletsBound.argtypes = [c.c_size_t]*3
    library.meshopt_buildMeshletsBound.restype = c.c_size_t
    library.meshopt_buildMeshlets.argtypes = [c.POINTER(Meshlet), c.POINTER(c.c_uint), c.POINTER(c.c_ubyte),
        c.POINTER(c.c_uint), c.c_size_t, c.POINTER(c.c_float), c.c_size_t, c.c_size_t, c.c_size_t, c.c_size_t, c.c_float]
    library.meshopt_buildMeshlets.restype = c.c_size_t
    library.meshopt_optimizeMeshlet.argtypes = [c.POINTER(c.c_uint), c.POINTER(c.c_ubyte), c.c_size_t, c.c_size_t]
    library.meshopt_optimizeMeshlet.restype = None
    positions = [(x/16*1.6-.8, y/16*1.6-.8, .5) for y in range(17) for x in range(17)]
    indices = []
    for y in range(16):
        for x in range(16):
            i = y*17+x
            indices += [i,i+1,i+18,i,i+18,i+17]
    grid_source = struct.pack(f'<{len(positions)*3}f', *(v for p in positions for v in p)) + struct.pack(f'<{len(indices)}I', *indices)
    if args.fixture == 'quad':
        positions = [(-.8,-.8,.5),(.8,-.8,.5),(.8,.8,.5),(-.8,.8,.5)]
        indices = [0,1,2,0,2,3]
    if args.fixture == 'triangle':
        positions = [(-.7,.6,.5),(.7,.6,.5),(0,-.6,.5)]
        indices = [0,1,2]
    if args.fixture == 'terrain16':
        positions = [(x/16*1.6-.8,y/16*1.6-.8,.5+.12*math.sin(math.pi*x/16)*math.sin(2*math.pi*y/16)) for y in range(17) for x in range(17)]
    if args.fixture == 'occlusion':
        positions = [(x,y,z) for z in (.65,.35) for x,y in ((-.6,-.6),(.6,-.6),(.6,.6),(-.6,.6))]
        indices = [0,1,2,0,2,3,4,5,6,4,6,7]
    imported = None
    if args.geometry:
        with args.geometry.open('rb') as stream:
            normalized = stream.read(64*1024*1024+1)
        if len(normalized)>64*1024*1024:
            parser.error('Normalized JSON exceeds 64 MiB')
        imported = json.loads(normalized)
        if type(imported.get('schema')) is not int or imported['schema'] not in (1,2,3) or not isinstance(imported.get('provenance'),dict):
            parser.error('Unsupported normalized geometry schema/provenance')
        positions = imported.get('positions',[])
        indices = imported.get('indices',[])
        if not 0<len(positions)<=8192 or not 0<len(indices)<=6144 or len(indices)%3:
            parser.error('Normalized geometry exceeds count limits')
        if any(not isinstance(p,list) or len(p)!=3 or any(type(v) not in (int,float) or not math.isfinite(v) or not math.isfinite(c.c_float(v).value) for v in p) for p in positions):
            parser.error('Invalid finite float3 normalized positions')
        if any(type(i) is not int or not 0<=i<len(positions) for i in indices):
            parser.error('Normalized triangle indices out of bounds')
    source = struct.pack(f'<{len(positions)*3}f', *(v for p in positions for v in p)) + struct.pack(f'<{len(indices)}I', *indices)
    vertices = (c.c_float * (len(positions)*3))(*(v for p in positions for v in p))
    lod = None
    original_source_digest = hashlib.sha256(source).hexdigest()
    if args.simplify_ratio is not None:
        library.meshopt_simplify.argtypes = [c.POINTER(c.c_uint),c.POINTER(c.c_uint),c.c_size_t,c.POINTER(c.c_float),c.c_size_t,c.c_size_t,c.c_size_t,c.c_float,c.c_uint,c.POINTER(c.c_float)]
        library.meshopt_simplify.restype = c.c_size_t
        destination = (c.c_uint*len(indices))()
        input_indices = (c.c_uint*len(indices))(*indices)
        reported_error = c.c_float()
        target = max(3,int(len(indices)*args.simplify_ratio)//3*3)
        # Official 1.3: LockBorder=1, ErrorAbsolute=4; output capacity is original index_count.
        result = library.meshopt_simplify(destination,input_indices,len(indices),vertices,len(positions),12,target,args.simplify_error,5,c.byref(reported_error))
        if not 0 < result < len(indices) or result % 3:
            raise RuntimeError('Simplifier did not produce a reduced triangle list')
        if not math.isfinite(reported_error.value) or reported_error.value > args.simplify_error+1e-6:
            raise RuntimeError('Native simplification error exceeds configured limit')
        canonical_positions = [tuple(vertices[i*3+axis] for axis in range(3)) for i in range(len(positions))]
        simplified = list(destination[:result])
        lod = qualify(canonical_positions,indices,simplified,args.max_height_error)
        lod.update(target_ratio=args.simplify_ratio,target_triangles=target//3,native_absolute_error_limit=args.simplify_error,native_reported_absolute_error=reported_error.value,options='LockBorder|ErrorAbsolute')
        indices = simplified
        source = struct.pack(f'<{len(positions)*3}f', *vertices)+struct.pack(f'<{len(indices)}I', *indices)
    source_indices = (c.c_uint*len(indices))(*indices)
    bound = library.meshopt_buildMeshletsBound(len(indices),64,64)
    meshlets = (Meshlet*bound)()
    # Official 1.3 API documents index_count as worst-case space for both outputs.
    refs = (c.c_uint*len(indices))()
    triangles = (c.c_ubyte*len(indices))()
    count = library.meshopt_buildMeshlets(meshlets,refs,triangles,source_indices,len(indices),vertices,len(positions),12,64,64,0)
    if not 0 < count <= bound:
        raise RuntimeError('Invalid native meshlet count')
    reconstructed = []
    for m in meshlets[:count]:
        library.meshopt_optimizeMeshlet(c.cast(c.byref(refs,m.vertex_offset*4),c.POINTER(c.c_uint)),
            c.cast(c.byref(triangles,m.triangle_offset),c.POINTER(c.c_ubyte)),m.triangle_count,m.vertex_count)
        if args.fixture == 'occlusion':
            if count != 1:
                raise RuntimeError('Occlusion order qualification requires one meshlet')
            ordered = [tuple(triangles[m.triangle_offset+i*3+axis] for axis in range(3)) for i in range(m.triangle_count)]
            ordered.sort(key=lambda t: positions[refs[m.vertex_offset+t[0]]][2],reverse=True)
            for i, triangle in enumerate(ordered):
                for axis in range(3):
                    triangles[m.triangle_offset+i*3+axis] = triangle[axis]
        for triangle in range(m.triangle_count):
            reconstructed.append(tuple(refs[m.vertex_offset + triangles[m.triangle_offset+triangle*3+axis]] for axis in range(3)))
    expected_triangles = [tuple(indices[i:i+3]) for i in range(0,len(indices),3)]
    if Counter(map(canonical,reconstructed)) != Counter(map(canonical,expected_triangles)):
        raise RuntimeError('Meshlet construction changed coverage or winding')
    if args.fixture == 'occlusion' and [round(positions[t[0]][2],2) for t in reconstructed] != [.65,.65,.35,.35]:
        raise RuntimeError('Occlusion qualification requires near triangles before far triangles')
    used_refs = max(m.vertex_offset+m.vertex_count for m in meshlets[:count])
    used_triangles = max(m.triangle_offset+m.triangle_count*3 for m in meshlets[:count])
    metadata = {
        'producer': 'meshoptimizer-1.3/buildMeshlets+optimizeMeshlet',
        'source_archive_sha256': archive_digest,
        'library_sha256': hashlib.sha256(args.library.read_bytes()).hexdigest(),
        'compiler': subprocess.check_output(['c++','--version'],text=True).splitlines()[0],
        'build': 'CMake Release; MESHOPT_BUILD_SHARED_LIBS=ON',
        'source_sha256': hashlib.sha256(source).hexdigest(), 'recipe': args.fixture+'-v1',
        'target': 'Vulkan1.3/EXT_mesh_shader; positions-f32x3/local-indices-u8',
        'max_vertices': 64, 'max_triangles': 64, 'cone_weight': 0, 'schema': 1,
    }
    if args.fixture == 'quad':
        metadata['fallback_for_source_sha256'] = hashlib.sha256(grid_source).hexdigest()
    if args.fixture == 'occlusion':
        metadata['qualification_triangle_order'] = 'near-z0.65-before-far-z0.35'
    if imported is not None:
        metadata['recipe'] = 'normalized-static-gltf-v1'
        metadata['normalization'] = imported['provenance']
        metadata['normalized_json_sha256'] = hashlib.sha256(normalized).hexdigest()
        if imported['schema'] in (2,3):
            metadata['recipe'] = 'normalized-static-gltf-unlit-v1'
            metadata['material_mapping'] = {'policy':'winding-preserving cyclic index triangle identity; reject conflicting duplicates',
                'meshlet_cooker_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'remap_source_sha256':hashlib.sha256(Path(__file__).with_name('remap_materials.py').read_bytes()).hexdigest()}
    if lod is not None:
        metadata['fallback_for_source_sha256'] = original_source_digest
        metadata['producer'] = 'meshoptimizer-1.3/simplify+buildMeshlets+optimizeMeshlet'
        metadata['recipe'] = args.fixture+'-lod-v1'
        metadata['lod_qualification'] = lod
    meta = json.dumps(metadata, sort_keys=True, separators=(',',':')).encode()
    payload = meta + bytes((-len(meta)) % 4)
    payload += struct.pack(f'<{len(positions)*3}f', *vertices)
    payload += b''.join(struct.pack('<4I',m.vertex_offset,m.triangle_offset,m.vertex_count,m.triangle_count) for m in meshlets[:count])
    payload += struct.pack(f'<{used_refs}I',*refs[:used_refs])
    payload += bytes(triangles[:used_triangles]) + bytes((-used_triangles)%4)
    header = struct.pack('<8s9I32s',b'THEIAMES',1,len(meta),len(positions),count,used_refs,used_triangles,64,64,len(payload),hashlib.sha256(payload).digest())
    blob = header + payload
    identifier = hashlib.sha256(blob).hexdigest()
    args.output_dir.mkdir(parents=True,exist_ok=True)
    material_report = None
    material_blob = None
    if imported is not None and imported['schema'] in (2,3):
        colors = imported.get('base_colors')
        assignments = imported.get('triangle_materials')
        if not isinstance(assignments,list):
            raise ValueError('Normalized unlit geometry requires triangle material assignments')
        mapped = remap(expected_triangles,assignments,reconstructed)
        material_source = json.dumps({'schema':1,'geometry_sha256':identifier,'base_colors':colors,'triangle_materials':mapped},sort_keys=True,separators=(',',':')).encode()
        material_blob = cook_material(blob,material_source)
        material_id = hashlib.sha256(material_blob).hexdigest()
        material_path = args.output_dir/(material_id+'.theia-material')
        material_report = {'path':str(material_path),'content_id':material_id,'bytes':len(material_blob),
                           'geometry_id':identifier,'materials':len(colors),'triangle_materials':mapped,
                           'mapping':'winding-preserving cyclic index triangle identity; reject conflicting duplicate assignments'}
    uv_report = None
    uv_blob = None
    if imported is not None and imported['schema'] == 3:
        uv_source = json.dumps({'schema':1,'geometry_sha256':identifier,'texcoords':imported.get('texcoords')},sort_keys=True,separators=(',',':')).encode()
        uv_blob = cook_uv(blob,uv_source)
        uv_id = hashlib.sha256(uv_blob).hexdigest()
        uv_path = args.output_dir/(uv_id+'.theia-uv')
        uv_report = {'path':str(uv_path),'content_id':uv_id,'bytes':len(uv_blob),'geometry_id':identifier,'vertices':len(positions)}
    destination = args.output_dir / (identifier+'.theia-mesh')
    destination.write_bytes(blob)
    if material_blob is not None:
        material_path.write_bytes(material_blob)
    if uv_blob is not None:
        uv_path.write_bytes(uv_blob)
    report = {'path': str(destination), 'content_id': identifier, 'bytes': len(blob), 'vertices': len(positions),
              'meshlets': count, 'triangles': len(reconstructed), 'coverage_and_winding_verified': True, 'metadata': metadata}
    if uv_report is not None:
        report['uv'] = uv_report
    if material_report is not None:
        report['materials'] = material_report
    if lod is not None:
        report['lod'] = lod
    suffix = '-lod' if lod is not None else ''
    name = 'imported' if imported is not None else args.fixture+suffix
    (args.output_dir/(name+'-report.json')).write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__ == '__main__':
    main()
