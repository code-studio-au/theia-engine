#!/usr/bin/env python3
"""Compile and validate the M0-D Slang shaders; publish only after all pass."""
import argparse
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--slangc', default='slangc', help='Slang compiler executable')
    parser.add_argument('--spirv-val', default='spirv-val', help='SPIR-V validator executable')
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'build' / 'shaders')
    parser.add_argument('--shared-primitive-interface', action='store_true',
                        help='qualify Slang shared-vertex mesh + GLSL perprimitiveEXT fragment fallback')
    parser.add_argument('--glslang', default='glslangValidator', help='GLSL fallback compiler executable')
    parser.add_argument('--spirv-dis', default='spirv-dis', help='shared-interface qualification disassembler')
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    shaders = [('authority.slang', 'authorityMesh', 'mesh', 'authority.mesh.spv'),
               ('authority.slang', 'authorityFragment', 'fragment', 'authority.frag.spv'),
               ('triangle.slang', 'meshMain', 'mesh', 'triangle.mesh.spv'),
               ('triangle.slang', 'fragmentMain', 'fragment', 'triangle.frag.spv'),
               ('amplified.slang', 'taskMain', 'amplification', 'triangle.task.spv'),
               ('amplified.slang', 'amplifiedMeshMain', 'mesh', 'triangle.amplified.mesh.spv'),
               ('indirect.slang', 'generateWork', 'compute', 'work.compute.spv'),
               ('indirect.slang', 'indirectTaskMain', 'amplification', 'triangle.indirect.task.spv'),
               ('spatial.slang', 'spatialCompute', 'compute', 'spatial.compute.spv'),
               ('spatial.slang', 'spatialTask', 'amplification', 'spatial.task.spv'),
               ('spatial.slang', 'spatialMesh', 'mesh', 'spatial.mesh.spv'),
               ('visibility.slang', 'visibilityTask', 'amplification', 'visibility.task.spv'),
               ('visibility.slang', 'visibilityMesh', 'mesh', 'visibility.mesh.spv'),
               ('visibility.slang', 'visibilityFragment', 'fragment', 'visibility.frag.spv'),
               ('resolve.slang', 'resolveMesh', 'mesh', 'resolve.mesh.spv'),
               ('resolve.slang', 'resolveFragment', 'fragment', 'resolve.frag.spv'),
               ('asset.slang', 'assetCompute', 'compute', 'asset.compute.spv'),
               ('asset.slang', 'assetTask', 'amplification', 'asset.task.spv'),
               ('asset.slang', 'assetMesh', 'mesh', 'asset.mesh.spv'),
               ('asset_camera.slang', 'assetCameraMesh', 'mesh', 'asset.camera.mesh.spv'),
               ('asset_lod.slang', 'assetLodCompute', 'compute', 'asset.lod.compute.spv'),
               ('asset_lod.slang', 'assetLodMesh', 'mesh', 'asset.lod.mesh.spv'),
               ('asset_cull.slang', 'assetCullCompute', 'compute', 'asset.cull.compute.spv'),
               ('asset_cull.slang', 'assetCulledMesh', 'mesh', 'asset.cull.mesh.spv'),
               ('asset_visibility.slang', 'assetVisibilityMesh', 'mesh', 'asset.visibility.mesh.spv'),
               ('asset_visibility.slang', 'assetVisibilityFragment', 'fragment', 'asset.visibility.frag.spv'),
               ('asset_resolve.slang', 'assetResolveFragment', 'fragment', 'asset.resolve.frag.spv'),
               ('asset_authored_resolve.slang', 'assetAuthoredResolveFragment', 'fragment', 'asset.authored.resolve.frag.spv'),
               ('asset_uv_resolve.slang', 'assetUvResolveFragment', 'fragment', 'asset.uv.resolve.frag.spv'),
               ('asset_texture_resolve.slang', 'assetTextureResolveFragment', 'fragment', 'asset.texture.resolve.frag.spv'),
               ('asset_texture_auto_resolve.slang', 'assetTextureAutoResolveFragment', 'fragment', 'asset.texture.auto.resolve.frag.spv'),
               ('asset_texture_table_resolve.slang', 'assetTextureTableResolveFragment', 'fragment', 'asset.texture.table.resolve.frag.spv')]
    try:
        subprocess.run([args.slangc, '-version'], check=True)
        if args.shared_primitive_interface:
            subprocess.run([args.glslang, '--version'], check=True)
        with tempfile.TemporaryDirectory(dir=args.output_dir) as temporary:
            for source, entry, stage, filename in shaders:
                output = Path(temporary) / filename
                if args.shared_primitive_interface and filename == 'asset.visibility.mesh.spv':
                    source = 'asset_visibility_shared.slang'
                if args.shared_primitive_interface and filename == 'asset.visibility.frag.spv':
                    subprocess.run([args.glslang, '-V', '--target-env', 'vulkan1.3',
                                    str(ROOT / 'shaders/render/asset_visibility_shared.frag'), '-o', str(output)], check=True)
                    subprocess.run([args.spirv_val, '--target-env', 'vulkan1.3', str(output)], check=True)
                    print(f'Compiled and validated specific GLSL fallback: {filename}', flush=True)
                    continue
                command = [args.slangc, str(ROOT / 'shaders/render' / source),
                           '-entry', entry, '-stage', stage, '-target', 'spirv',
                           '-profile', 'spirv_1_6', '-o', str(output)]
                if filename == 'asset.lod.mesh.spv':
                    command += ['-D', 'THEIA_LOD_MESH=1']
                if stage in ('mesh', 'amplification'):
                    command += ['-capability', 'spvMeshShadingEXT']
                subprocess.run(command, check=True)
                subprocess.run([args.spirv_val, '--target-env', 'vulkan1.3', str(output)], check=True)
                print(f'Compiled and validated {stage}: {filename}', flush=True)
            if args.shared_primitive_interface:
                for filename in ('asset.visibility.mesh.spv', 'asset.visibility.frag.spv'):
                    assembly = subprocess.check_output([args.spirv_dis, str(Path(temporary) / filename)], text=True)
                    per_primitive = set()
                    location_zero = set()
                    for line in assembly.splitlines():
                        tokens = line.split()
                        if len(tokens) >= 3 and tokens[0] == 'OpDecorate':
                            if tokens[2] == 'PerPrimitiveEXT': per_primitive.add(tokens[1])
                            if tokens[2:] == ['Location', '0']: location_zero.add(tokens[1])
                    if not per_primitive.intersection(location_zero) or 'OpCapability Geometry' in assembly:
                        raise ValueError(f'{filename}: unqualified per-primitive interface or Geometry capability')
                print('Shared primitive interface: matching Location 0 PerPrimitiveEXT; no Geometry capability', flush=True)
            for _, _, _, filename in shaders:
                (Path(temporary) / filename).replace(args.output_dir / filename)
        print(f'Shaders ready in {args.output_dir}')
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'Shader qualification failed: {error}\n')


if __name__ == '__main__':
    main()
