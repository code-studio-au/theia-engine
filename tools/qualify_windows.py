#!/usr/bin/env python3
"""Cross-link the repository's Rust CI matrix with a private MinGW SDK.
Optional Wine executes CPU/library tests, never qualifies native Windows/GPU.
Requires rustup's x86_64-pc-windows-gnu target and extracted Ubuntu SDK packages.
"""
import argparse
import json
import os
import platform
from pathlib import Path
import re
import subprocess
import sys


def qualify(sdk,output,wine=False):
    if not sys.platform.startswith('linux') or platform.machine()!='x86_64':raise RuntimeError('This private SDK harness requires Linux/x86-64')
    sdk,output=sdk.resolve(),output.resolve();output.mkdir(parents=True,exist_ok=True)
    compiler=sdk/'usr/bin/x86_64-w64-mingw32-gcc-posix'
    if not compiler.is_file():raise RuntimeError('Extracted MinGW compiler missing')
    env=os.environ.copy();env['PATH']=str(sdk/'usr/bin')+os.pathsep+env.get('PATH','')
    env['CC_x86_64_pc_windows_gnu']=str(compiler)
    env['AR_x86_64_pc_windows_gnu']=str(sdk/'usr/bin/x86_64-w64-mingw32-ar')
    env['CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER']=str(compiler)
    target=Path('build/qualification/windows-target').resolve();env['CARGO_TARGET_DIR']=str(target)
    workflow=Path('.github/workflows/rust.yml').read_text()
    match=re.search(r'package: \[([^\]]+)\]',workflow)
    if match is None:raise RuntimeError('No Rust package matrix')
    packages=[value.strip() for value in match.group(1).split(',')]
    reports=[];executables=[]
    for package in packages:
        name=package.replace('/','-');manifest=package+'/Cargo.toml'
        args=['--locked','--manifest-path',manifest,'--target','x86_64-pc-windows-gnu']
        with (output/(name+'-clippy.log')).open('w') as log:
            subprocess.run(['cargo','clippy',*args,'--all-targets','--','-D','warnings'],env=env,
                stdout=log,stderr=subprocess.STDOUT,check=True)
        with (output/(name+'-normal-build.log')).open('w') as log:
            subprocess.run(['cargo','build',*args,'--all-targets'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
        with (output/(name+'-tests.jsonl')).open('w') as log, (output/(name+'-build.log')).open('w') as errors:
            subprocess.run(['cargo','test',*args,'--no-run','--message-format=json'],env=env,
                stdout=log,stderr=errors,check=True)
        tests=[]
        for line in (output/(name+'-tests.jsonl')).read_text().splitlines():
            item=json.loads(line)
            if item.get('reason')=='compiler-artifact' and item.get('profile',{}).get('test') and item.get('executable'):
                tests.append(item['executable'])
        executables += [(name,path) for path in tests]
        reports.append({'package':package,'windows_cross_link':True,'denied_warning_clippy':True,'test_executables':tests})
        print('Windows GNU cross-linked and linted:',package,flush=True)
    if wine:
        wine_env=os.environ.copy();wine_env['WINEPREFIX']=str(output/'wine-prefix');wine_env['WINEARCH']='win64'
        # Scope compatibility runtime noise to its own prefix; Rust test assertions
        # and process exits are the success contract, no Vulkan diagnostics claimed.
        wine_env['WINEDEBUG']='-all'
        try:
            with (output/'wine-initialize.log').open('w') as log:
                subprocess.run(['wineboot','--init'],env=wine_env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=45)
            for name,path in executables:
                with (output/(name+'-'+Path(path).stem+'-wine.log')).open('w') as log:
                    subprocess.run(['wine',path,'--test-threads=1'],env=wine_env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=45)
                print('Wine compatibility tests passed:',name,Path(path).name,flush=True)
        finally:
            subprocess.run(['wineserver','-k'],env=wine_env,check=True,timeout=10)
    (output/'report.json').write_text(json.dumps({'target':'x86_64-pc-windows-gnu','packages':reports,
        'wine_compatibility_tests':wine,'native_windows':False,'windows_gpu_execution':False},indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sdk',type=Path,default=Path('build/qualification/windows-tool/root'))
    parser.add_argument('--output',type=Path,default=Path('build/qualification/windows-build'))
    parser.add_argument('--wine',action='store_true')
    args=parser.parse_args();qualify(args.sdk,args.output,args.wine)
