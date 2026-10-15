#!/usr/bin/env python3
"""Cook and independently admit the standard eight-graph streamed benchmark.

Uses the qualified offline tool cache; does not download/install dependencies.
Supplying --cooked-scene to the launcher reuses an already prepared manifest.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]


def prepare(output,skip_build=False):
    output=Path(output).resolve();output.mkdir(parents=True,exist_ok=True)
    suffix='.exe' if os.name=='nt' else ''
    importer=ROOT/'tools/theia-gltf-import/target/debug'/('theia-gltf-import'+suffix)
    native={
        '--library':ROOT/'build/qualification/meshoptimizer/lib-build'/('meshoptimizer.dll' if os.name=='nt' else 'libmeshoptimizer.so'),
        '--meshoptimizer-source':ROOT/'build/qualification/meshoptimizer/meshoptimizer-1.3.tar.gz',
        '--ktx':ROOT/'build/qualification/ktx-tool/source-build/Release'/('ktx'+suffix),
        '--ktx-build':ROOT/'build/qualification/ktx-tool/source-build',
        '--ktx-source':ROOT/'build/qualification/ktx-tool/ktx-source-v4.4.2.tar.gz',
    }
    missing=[str(path) for path in native.values() if not path.exists()]
    if skip_build and not importer.is_file():missing.append(str(importer))
    if missing:raise RuntimeError('Missing qualified offline tools: '+', '.join(missing)+'. See docs/SCENE_BENCHMARK.md or supply --cooked-scene with eight prepared graphs.')
    proof=output/'proof';proof.mkdir(exist_ok=True)
    (proof/'report.json').unlink(missing_ok=True)
    with (output/'prepare.log').open('w') as log:
        if not skip_build:
            subprocess.run(['cargo','build','--locked','--manifest-path',str(ROOT/'tools/theia-gltf-import/Cargo.toml')],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
        flags=sum(([flag,str(path)] for flag,path in native.items()),[])
        subprocess.run([sys.executable,str(ROOT/'tools/cook_benchmark_scene.py'),'--output',str(output),
            '--chunk-fallbacks','--importer',str(importer),*flags],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
        subprocess.run([sys.executable,str(ROOT/'tools/qualify_benchmark_scene.py'),'--scene',str(output/'scene.json'),
            '--output',str(proof)],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
    result=json.loads((proof/'report.json').read_bytes())
    if not result['passed'] or len(result.get('fallbacks',[]))!=4:raise RuntimeError('Streamed benchmark source/admission proof is incomplete')
    return output/'scene.json'


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--skip-build',action='store_true',help='reuse the already built importer; asset admission still invokes its Rust CPU checker')
    args=parser.parse_args()
    try:print(prepare(args.output,args.skip_build))
    except (OSError,RuntimeError,subprocess.SubprocessError) as error:parser.exit(1,str(error)+'\n')
