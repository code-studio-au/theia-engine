#!/usr/bin/env python3
"""Native compute gate for optional nonuniform storage-buffer descriptor arrays."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from headless_display import owned_display


def require(value,message):
    if not value:
        raise RuntimeError(message)


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';result.write_text('{"passed":false}\n')
    shader=Path('build/shaders/world.descriptors.compute.spv')
    assembly=subprocess.check_output(['spirv-dis',str(shader)],text=True)
    require(all('OpCapability '+cap in assembly for cap in ('ShaderNonUniform','RuntimeDescriptorArray',
            'StorageBufferArrayNonUniformIndexing')),'Missing storage descriptor capabilities')
    require(any('OpDecorate' in line and 'NonUniform' in line for line in assembly.splitlines()),
            'Missing nonuniform decoration')
    (out/'shader.spvasm').write_text(assembly)
    omit=out/'without-probe';omit.mkdir(exist_ok=True)
    for source in Path('build/shaders').glob('*.spv'):
        if source.name!=shader.name:
            target=omit/source.name
            if not target.exists():target.symlink_to(source.resolve())
    scene=json.loads(args.scene.read_bytes());mesh=scene['quadrants'][0]['cook']['path']
    cases=[
        ('default',['--world-descriptor-test'],True,False),
        ('shared',['--world-descriptor-test','--shader-dir',str(args.shared_shaders.resolve())],True,False),
        ('missing-shader',['--world-descriptor-test','--shader-dir',str(omit)],False,False),
        ('normal-without-probe',['--asset',mesh,'--material-test','--frames','3','--shader-dir',str(omit)],True,False),
        ('conflicting-benchmark',['--world-descriptor-test','--benchmark'],False,True),
        ('conflicting-world',['--world-descriptor-test','--world-upload-test',str(args.scene.resolve())],False,True),
    ]
    report=dict(passed=False,cases=[])
    try:
        with owned_display(out/'display',args.headless_dri_prime) as (env,_,display):
            report['display']=display
            for name,flags,success,preflight in cases:
                log_path=out/(name+'.log')
                with log_path.open('w') as log:
                    process=subprocess.run([str(args.renderer.resolve()),*flags,'--gpu','7900'],env=env,
                        stdout=log,stderr=subprocess.STDOUT,timeout=60)
                text=log_path.read_text()
                require((process.returncode==0)==success,name+' unexpected exit')
                if preflight:
                    require('Selected GPU:' not in text,name+' reached Vulkan')
                else:
                    require('VK_LAYER_KHRONOS_validation' in text and
                        'Validation totals (including teardown): 0 warnings, 0 errors' in text,
                        name+' missing actual validation/clean teardown')
                if success:require('Presented 3 frames' in text,name+' presentation incomplete')
                row=dict(case=name,exit=process.returncode)
                if name in ('default','shared'):
                    prefix='World storage descriptor probe verified: '
                    proof=[json.loads(line[len(prefix):]) for line in text.splitlines() if line.startswith(prefix)]
                    require(len(proof)==1 and proof[0]['verified_gpu_values']==64 and proof[0]['source_buffers']==4
                        and proof[0]['nonuniform_within_workgroup'] and not proof[0]['rendered_world_chunks'],
                        name+' incomplete GPU gate')
                    row['proof']=proof[0]
                report['cases'].append(row)
                print('Verified storage descriptor case:',name,flush=True)
        report.update(passed=True,shader_sha256=hashlib.sha256(shader.read_bytes()).hexdigest(),
            shared_shader_sha256=hashlib.sha256((args.shared_shaders/shader.name).read_bytes()).hexdigest(),
            fragment_stage_qualified=False,world_renderer_implemented=False)
    finally:
        result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--shared-shaders',type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
