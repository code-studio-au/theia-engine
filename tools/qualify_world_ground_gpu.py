#!/usr/bin/env python3
"""Native validation and source-oracle qualification for batched GPU terrain heights."""
import argparse
import json
from pathlib import Path
import subprocess
from headless_display import owned_display
from qualify_world_ground import qualify as qualify_cpu, require


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    report=dict(passed=False,cases=[],rendered_proxies=False);result=out/'report.json'
    result.write_text(json.dumps(report)+'\n')
    try:
        cpu_out=out/'source-reference'
        qualify_cpu(argparse.Namespace(scene=args.scene,output=cpu_out,renderer=args.renderer))
        reference=json.loads((cpu_out/'reference.json').read_bytes())
        expected=[height for row in reference['heights'] for height in row]
        empty=out/'empty.json';empty.write_text('[]\n')
        outside=out/'outside.json';outside.write_text('[[65,0]]\n')
        shader_dir=out/'missing-shader';shader_dir.mkdir(exist_ok=True)
        exe=str(args.renderer.resolve());scene=str(args.scene.resolve());points=str(cpu_out/'points.json')
        with owned_display(out/'display',args.headless_dri_prime) as (env,_,display):
            report['display']=display
            cases=[('full',points,[],True),('shared',points,['--shader-dir',str(args.shared_shaders.resolve())],True),
                ('empty',str(empty),[],False),('outside',str(outside),[],False),
                ('missing-scene',points,[],False),('missing-shader',points,['--shader-dir',str(shader_dir)],False),
                ('descriptor-regression',None,[],True)]
            for name,query,extra,succeeds in cases:
                gpu_report=out/(name+'-gpu.json');log_path=out/(name+'.log')
                command=['--world-descriptor-test'] if query is None else ['--world-ground-gpu-test',
                    str(out/'missing.json') if name=='missing-scene' else scene,query,str(gpu_report)]
                with log_path.open('w') as log:
                    proc=subprocess.run([exe,*command,*extra,'--gpu','7900'],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=90)
                text=log_path.read_text()
                require((proc.returncode==0)==succeeds and 'VK_LAYER_KHRONOS_validation' in text and
                    'Validation totals (including teardown): 0 warnings, 0 errors' in text,name+' unexpected exit/diagnostics')
                row=dict(case=name,exit=proc.returncode)
                if succeeds:
                    require('Presented 3 frames' in text,name+' incomplete clear presentation')
                    if query is not None:
                        data=json.loads(gpu_report.read_bytes())
                        require(data['passed'] and data['gpu_execution'] and not data['rendered_proxies'] and
                            len(data['heights'])==len(expected),'Wrong GPU result shape/scope')
                        largest=max(abs(a-b) for a,b in zip(data['heights'],expected))
                        require(largest<3e-6,'GPU heights differ from independently qualified source heights')
                        row.update(verified_heights=len(expected),max_source_qualified_error=largest,
                            table_bytes=data['table_bytes'],physical_buffer_bytes=data['physical_buffer_bytes'])
                    else:require('"verified_gpu_values":64' in text,'Descriptor guard regression incomplete')
                else:require(not gpu_report.exists(),'Failure falsely completed GPU report')
                report['cases'].append(row);print('Verified selected terrain GPU:',name,flush=True)
            for name,flags in [('conflict-benchmark',['--benchmark']),('conflict-smoke',['--wsi-smoke-test']),('conflict-render',['--world-descriptor-test'])]:
                proc=subprocess.run([exe,'--world-ground-gpu-test',scene,points,str(out/'unused.json'),*flags],
                    env=env,capture_output=True,text=True,timeout=15)
                require(proc.returncode!=0 and 'Selected GPU:' not in proc.stdout,'Conflict reached Vulkan')
                report['cases'].append(dict(case=name,exit=proc.returncode))
        report.update(passed=True,gpu_execution=True,source_reference=str(cpu_out/'report.json'))
    finally:result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('scene','output','shared-shaders'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--headless-dri-prime',default='pci-0000_03_00_0')
    qualify(parser.parse_args())
