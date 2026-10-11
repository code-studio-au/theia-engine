#!/usr/bin/env python3
"""Qualify a localhost authority listener and two separate Vulkan viewers.
Requires a qualified desktop/GPU; this is not a headless CI test.
Only owned child processes, public certificates and logs are managed.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def verify_faults(log):
    import re
    match = re.search(r'Network fault relay: Report \{ forwarded: (\d+), dropped: (\d+), delayed: (\d+), reordered: (\d+), duplicated: (\d+), max_pending: (\d+) \}', log)
    if match is None:
        raise RuntimeError('Missing bounded fault report: '+log)
    counts = list(map(int, match.groups()))
    if not all(v>0 for v in counts[:5]) or not 1<=counts[5]<=2:
        raise RuntimeError('Faults were not exercised within bounds: '+str(counts))


def qualify(output, laboratory=False, wsi=False, faults=False, repair=False):
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    suffix = '.exe' if os.name == 'nt' else ''
    server_exe = Path('apps/theia-network-lab/target/debug/theia-network-lab'+suffix).resolve()
    viewer_exe = Path('apps/theia-render-lab/target/debug/theia-render-lab'+suffix).resolve()
    processes, handles = [], []
    try:
        server_log = output/'server.log'
        handles.append(server_log.open('w'))
        arguments = [str(server_exe), '--serve', '--certificate', str(output/'localhost.der')]
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
            raise RuntimeError('Listener readiness failed: '+server_log.read_text())
        clients = []
        for index in range(2):
            path = output/f'client-{index}.log'
            handles.append(path.open('w'))
            arguments = [str(viewer_exe), '--network-client', ready['address'],
                '--network-certificate', ready['certificate']]
            if faults:
                arguments.append('--network-faults')
            if repair:
                arguments.append('--network-repair-test')
            arguments += ['--wsi-smoke-test'] if wsi else ['--frames', '180']
            process = subprocess.Popen(arguments, stdout=handles[-1], stderr=subprocess.STDOUT)
            processes.append(process)
            clients.append((process, path))
        roles = []
        for process, path in clients:
            if process.wait(timeout=30)!=0:
                raise RuntimeError('Viewer failed: '+path.read_text())
            log = path.read_text()
            for required in ['readonly snapshots', 'Validation totals (including teardown): 0 warnings, 0 errors',
                             'Presented 120 frames' if wsi else 'Presented 180 frames']:
                if required not in log:
                    raise RuntimeError('Missing '+required+': '+log)
            assigned = [role for role in [10,20] if f'Verified server assigned actor {role},' in log]
            if len(assigned)!=1:
                raise RuntimeError('Invalid server-owned identity: '+log)
            roles.extend(assigned)
            import re
            replication = re.search(r'Live replication: ReplicationStats \{ full: (\d+), delta: (\d+), acks_sent: (\d+), repairs_sent: (\d+), withheld_acks: (\d+) \}',log)
            if replication is None or not all(int(value)>0 for value in replication.groups()[:3]):
                raise RuntimeError('Viewer did not consume full/delta states and send snapshot ACKs: '+log)
            if repair and not all(int(value)>0 for value in replication.groups()[3:]):
                raise RuntimeError('ACK withholding/full repair not exercised: '+log)
            if faults:
                verify_faults(log)
        if sorted(roles)!=[10,20] or server.wait(timeout=10)!=0:
            raise RuntimeError('Pair shutdown or identity check failed: '+server_log.read_text())
        if repair:
            import re
            count = re.search(r'full_repairs: (\d+)',server_log.read_text())
            if count is None or int(count.group(1))==0:
                raise RuntimeError('Server did not consume a full repair request')
        print('Qualified separate encrypted GPU viewers:', roles, 'profile', ready['profile'],
              'WSI', wsi, 'faults', faults, 'repair', repair, '; logs', output)
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
    parser.add_argument('--output', type=Path, default=Path('build/qualification/m8-network-viewer'))
    parser.add_argument('--laboratory', action='store_true')
    parser.add_argument('--wsi', action='store_true')
    parser.add_argument('--faults', action='store_true')
    parser.add_argument('--repair', action='store_true')
    args = parser.parse_args()
    qualify(args.output, args.laboratory, args.wsi, args.faults, args.repair)
