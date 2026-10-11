#!/usr/bin/env python3
"""Qualify one localhost listener and two independent encrypted client processes.
Portable process orchestration; build the network lab first. Only generated
public certificates and qualification logs are written; no signing keys.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def qualify(output, laboratory=False, delay=0):
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    executable = Path('apps/theia-network-lab/target/debug/theia-network-lab').resolve()
    if os.name == 'nt':
        executable = executable.with_suffix('.exe')
    processes = []
    handles = []
    try:
        server_log = output/'server.log'
        handles.append(server_log.open('w'))
        arguments = [str(executable), '--serve', '--certificate', str(output/'localhost.der')]
        if laboratory:
            arguments.append('--laboratory-profile')
        server = subprocess.Popen(arguments, stdout=handles[-1], stderr=subprocess.STDOUT)
        processes.append(server)
        deadline = time.monotonic()+10
        ready = None
        while time.monotonic()<deadline and server.poll() is None:
            lines = server_log.read_text().splitlines()
            if lines:
                try:
                    candidate = json.loads(lines[0])
                    if candidate.get('listener'):
                        ready = candidate
                        break
                except json.JSONDecodeError:
                    pass
            time.sleep(.02)
        if ready is None:
            raise RuntimeError('Listener did not publish readiness: '+server_log.read_text())
        clients = []
        for index in range(2):
            if index and delay:
                time.sleep(delay)
            path = output/f'client-{index}.log'
            handles.append(path.open('w'))
            client = subprocess.Popen([str(executable), '--client-probe', ready['address'], ready['certificate']],
                stdout=handles[-1], stderr=subprocess.STDOUT)
            processes.append(client)
            clients.append((client,path))
        roles = []
        for client,path in clients:
            if client.wait(timeout=20)!=0:
                raise RuntimeError('Client probe failed: '+path.read_text())
            result = json.loads(path.read_text().splitlines()[-1])
            if not result['probe'] or result['profile']!=ready['profile'] or result['health']!=100 or result['applied_move']!=1:
                raise RuntimeError('Unexpected probe state: '+str(result))
            roles.append(result['actor'])
        if sorted(roles)!=[10,20] or server.wait(timeout=10)!=0:
            raise RuntimeError('Actor binding or listener shutdown failed: '+server_log.read_text())
        print('Qualified independent encrypted listener/client processes:', ready['address'], 'roles',roles,'profile',ready['profile'],'delay',delay,'seconds; logs',output)
    finally:
        for process in processes:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        for handle in handles:
            handle.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,default=Path('build/qualification/m8-listener-processes'))
    parser.add_argument('--laboratory',action='store_true')
    parser.add_argument('--second-client-delay',type=float,default=0)
    args = parser.parse_args()
    if not 0<=args.second_client_delay<=10:
        parser.error('Second client delay must be 0..10 seconds')
    qualify(args.output,args.laboratory,args.second_client_delay)
