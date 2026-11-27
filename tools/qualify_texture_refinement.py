#!/usr/bin/env python3
"""Qualify one asynchronous coarse-to-finer mip-tail publication and retention."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile
import time
from headless_display import owned_display


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference',type=Path,required=True)
    parser.add_argument('--texture-cook',type=Path,required=True)
    parser.add_argument('--qrenderdoc',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--initial-texels',type=int,default=507)
    parser.add_argument('--target-texels',type=int,default=508)
    parser.add_argument('--linear-filter',action='store_true')
    parser.add_argument('--shader-dir',type=Path,default=Path('build/shaders'))
    parser.add_argument('--headless-dri-prime')
    args=parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=True)
    report={'passed':False,'captures':[]}
    scene=json.loads(args.reference.read_bytes());texture=json.loads(args.texture_cook.read_bytes())
    exe=Path('apps/theia-render-lab/target/debug/theia-render-lab')
    base=[exe,'--gpu','7900','--asset',scene['path'],'--materials',scene['materials']['path'],
          '--uv-test',scene['uv']['path'],'--texture-auto-mips',texture['path'],
          '--texture-tail-budget',str(args.initial_texels),'--texture-refine-budget',str(args.target_texels),
          '--shader-dir',args.shader_dir]
    if args.linear_filter:base+=['--texture-linear-filter']

    def execute(command,name,env,succeeds=True):
        with (args.output/(name+'.log')).open('w') as log:
            result=subprocess.run(list(map(str,command)),env=env,stdout=log,stderr=subprocess.STDOUT,timeout=90)
        if (result.returncode==0)!=succeeds:raise RuntimeError(name+': unexpected exit')
        return (args.output/(name+'.log')).read_text(errors='replace')

    def validated(command,name,env):
        text=execute(command,name,env)
        if not all(value in text for value in ['Native surface created (Wayland)','Presented 120 frames',
            'Validation totals (including teardown): 0 warnings, 0 errors']):
            raise RuntimeError(name+': incomplete validation evidence')
        return text

    try:
        with owned_display(args.output/'display',args.headless_dri_prime) as (runtime_env,capture_env,info):
            report['headless_display']=info
            log=validated(base+['--wsi-smoke-test'],'wsi',runtime_env)
            if 'Mip refinement qualification: 1 publications, 0 budget deferrals, 0 source rejections' not in log:
                raise RuntimeError('Finer tail did not publish exactly once')
            charges=re.search(r'active (\d+) \+ pending (\d+) physical image bytes',log)
            if not charges:raise RuntimeError('Missing measured coexistence charge')
            active,pending=map(int,charges.groups());total=active+pending
            precision=None
            if args.linear_filter:
                precision=int(re.search(r'Texture filter precision: (\d+) fraction bits',log)[1])
            exact=validated(base+['--frames','120','--texture-budget',str(total)],'exact-budget',runtime_env)
            short=validated(base+['--frames','120','--texture-budget',str(total-1)],'short-budget',runtime_env)
            if '1 publications, 0 budget deferrals, 0 source rejections' not in exact or '0 publications, 1 budget deferrals, 0 source rejections' not in short:
                raise RuntimeError('Exact/short coexistence boundary did not publish/retain')
            report.update(active_image_bytes=active,pending_image_bytes=pending,coexistence_image_bytes=total)
            for name,frame,expected,budget in [('initial',10,args.initial_texels,total),
                    ('pending',21,args.initial_texels,total),('refined',50,args.target_texels,total),
                    ('deferred',50,args.initial_texels,total-1)]:
                env=capture_env.copy()
                env.update(THEIA_CAPTURE_OUTPUT=str(args.output/name),THEIA_CAPTURE_FRAME=str(frame),
                    THEIA_CAPTURE_ASSET=scene['path'],THEIA_CAPTURE_MATERIAL_PAGE=scene['materials']['path'],
                    THEIA_CAPTURE_UV_PAGE=scene['uv']['path'],THEIA_CAPTURE_TEXTURE_PAGE=texture['path'],
                    THEIA_CAPTURE_TEXTURE_AUTO_MIPS='1',THEIA_CAPTURE_TEXTURE_TAIL_BUDGET=str(args.initial_texels),
                    THEIA_CAPTURE_TEXTURE_REFINE_BUDGET=str(args.target_texels),THEIA_CAPTURE_EXPECT_TEXTURE_TAIL_BUDGET=str(expected),
                    THEIA_CAPTURE_TEXTURE_IMAGE_BUDGET=str(budget),THEIA_CAPTURE_SCENARIO='2',
                    THEIA_CAPTURE_SHADER_DIR=str(args.shader_dir.resolve()))
                if args.linear_filter:env.update(THEIA_CAPTURE_TEXTURE_LINEAR_FILTER='1',THEIA_CAPTURE_FILTER_PRECISION_BITS=str(precision))
                execute([args.qrenderdoc,'--python','tools/renderdoc_qualify.py'],name,env)
                captured=json.loads((args.output/name/'report.json').read_bytes())
                if not all(captured.get(field) for field in ['replay_verified','texture_mip_tail_verified','texture_automatic_mips_verified','texture_gpu_mips_verified']):
                    raise RuntimeError(name+': incomplete retained/refined replay')
                if name=='pending' and not any('vkCmdCopyBufferToImage' in action['name'] for action in captured['actions']):
                    raise RuntimeError('Pending capture has no finer image transfer; idle coarse rendering is insufficient evidence')
                report['captures'].append({'name':name,'frame':frame,'source_first':captured['texture_source_first_level'],
                    'resident_mips':captured['texture_mip_count'],'pixels':len(captured['camera_samples']),
                    'probes':len(captured['texture_footprint_probes'])})
            # Mutate only an owned clone after initialization; verified source stays untouched.
            with tempfile.TemporaryDirectory(prefix='theia-mip-reject-') as directory:
                clone=Path(directory)/Path(texture['path']).name
                original=Path(texture['path']).read_bytes();clone.write_bytes(original)
                command=list(map(str,base));command[command.index('--texture-auto-mips')+1]=str(clone)
                path=args.output/'changed-source.log'
                with path.open('w') as log:
                    child=subprocess.Popen(command+['--frames','120'],env=runtime_env,stdout=log,stderr=subprocess.STDOUT)
                    try:
                        deadline=time.monotonic()+10
                        while 'Mip refinement scheduler ready:' not in path.read_text():
                            if child.poll() is not None or time.monotonic()>deadline:raise RuntimeError('Source-rejection fixture initialization failed')
                            time.sleep(.001)
                        bad=bytearray(original);bad[-4]^=1;clone.write_bytes(bad)
                        if child.wait(timeout=30)!=0:raise RuntimeError('Source rejection did not retain active image')
                    finally:
                        if child.poll() is None:
                            child.kill();child.wait()
                text=path.read_text()
                if not all(value in text for value in ['0 publications, 0 budget deferrals, 1 source rejections',
                    'Presented 120 frames','Validation totals (including teardown): 0 warnings, 0 errors']):
                    raise RuntimeError('Changed source was not rejected with clean retained rendering')
            report.update(passed=True,exact_publication=True,short_budget_retained=True,changed_source_retained=True)
    except (OSError,ValueError,KeyError,RuntimeError,subprocess.SubprocessError) as error:
        report['error']=str(error);parser.exit(1,str(error)+'\n')
    finally:
        (args.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':main()
