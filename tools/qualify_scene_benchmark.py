#!/usr/bin/env python3
"""Qualify the bounded benchmark with separate Wayland validation and Xwayland replay."""
import argparse
import json
import os
from pathlib import Path
import subprocess
from headless_display import owned_display
from run_scene_benchmark import export

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--qrenderdoc',type=Path,required=True)
    parser.add_argument('--gpu',default='7900');parser.add_argument('--headless-dri-prime')
    args=parser.parse_args();args.output=args.output.resolve();args.output.mkdir(parents=True,exist_ok=True)
    exe=Path('apps/theia-render-lab/target/debug/theia-render-lab').resolve()
    report={'passed':False,'runtime':[],'captures':[],'negative':[]}
    def execute(command,path,env):
        with path.open('w') as log:
            subprocess.run(list(map(str,command)),env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
    try:
        invalid=[['--benchmark','--benchmark-players','10001'],['--benchmark','--frames','3','--benchmark-warmup','1'],
            ['--benchmark','--benchmark-warmup',str(2**64-1)],['--benchmark','--benchmark-seconds','NaN'],
            ['--benchmark','--benchmark-width','0'],['--benchmark','--visibility-test'],['--benchmark','--wsi-smoke-test'],['--benchmark-players','10']]
        for flags in invalid:
            result=subprocess.run([str(exe),*flags],capture_output=True,text=True,timeout=15)
            if not result.returncode or 'Native surface created' in result.stdout:raise RuntimeError('Invalid benchmark initialized Vulkan')
            report['negative'].append({'flags':flags,'error':result.stderr})
        with owned_display(args.output/'display',args.headless_dri_prime) as (env,capture_env,display):
            report['display']=display
            for players in [0,100,500,1000,10000]:
                output=args.output/f'runtime-{players}';output.mkdir(parents=True,exist_ok=True)
                path=output/'report.json';path.unlink(missing_ok=True)
                execute([exe,'--benchmark','--benchmark-players',players,'--frames','240','--benchmark-warmup','30',
                    '--benchmark-report',path,'--gpu',args.gpu],output/'run.log',env)
                text=(output/'run.log').read_text()
                if not all(value in text for value in ['Native surface created (Wayland)','Presented 240 frames','Validation totals (including teardown): 0 warnings, 0 errors']):
                    raise RuntimeError('Missing native Wayland validation evidence')
                timings=json.loads(path.read_bytes())
                if len(timings['samples'])!=240 or not timings['overlay'] or timings['samples'][-1]['scene_time']!=60:raise RuntimeError('Incomplete frame accounting/route')
                export(timings,path);report['runtime'].append({'players':players,'frames':240,'report':str(path)})
                print('Validated native Wayland population:',players,flush=True)
            for players,frame,hud in [(1000,10,False),(1000,90,False),(1000,170,False),(0,90,False),(100,170,False),(10000,170,False),(1000,90,True)]:
                output=args.output/f'replay-{players}-{frame}-hud{int(hud)}';output.mkdir(parents=True,exist_ok=True)
                replay_env=capture_env.copy();replay_env.update(THEIA_BENCHMARK_CAPTURE_OUTPUT=str(output),THEIA_BENCHMARK_PLAYERS=str(players),
                    THEIA_BENCHMARK_CAPTURE_FRAME=str(frame),THEIA_BENCHMARK_OVERLAY=str(int(hud)),THEIA_BENCHMARK_GPU=args.gpu)
                execute([args.qrenderdoc,'--python','tools/renderdoc_benchmark.py'],output/'capture.log',replay_env)
                captured=json.loads((output/'report.json').read_bytes())
                if not captured['passed'] or (hud and not captured.get('overlay_verified')):raise RuntimeError('Incomplete scene/stats replay')
                report['captures'].append(captured)
                print('Verified replay population/frame/overlay:',players,frame,hud,flush=True)
        report['passed']=True
    except (OSError,RuntimeError,ValueError,KeyError,subprocess.SubprocessError) as error:
        report['error']=str(error);parser.exit(1,str(error)+'\n')
    finally:(args.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
