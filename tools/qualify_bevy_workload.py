#!/usr/bin/env python3
"""Validate Bevy proxy poses against the existing motion oracle and independent source terrain."""
import argparse
import json
import math
from pathlib import Path
import subprocess
import struct

from qualify_world_ground import triangles, height
from world_camera_reference import require


def verify(fixture_path,bevy,theia,output):
    fixture=json.loads(fixture_path.read_bytes());scene=json.loads(Path(fixture['source_manifest']).read_bytes())
    original=json.loads(Path(fixture['source_report']).read_bytes());period=original['metadata']['camera_route_frames']
    surfaces=[]
    for group in ('quadrants','fallbacks'):
        for region,entry in enumerate(scene[group]):
            source=triangles(entry);origin=[(region%2-1)*64,(region//2-1)*64]
            cells=[]
            for cell in range(64):
                lo=[origin[0]+(cell%8)*8,origin[1]+(cell//8)*8]
                cells.append([t for t in source if all(max(p[a] for p in t)>lo[a] and min(p[a] for p in t)<lo[a]+8 for a in range(2))])
            surfaces.append((origin,cells))
    max_height=max_motion=0.;count=0;levels=set();details=set()
    frames={0,len(fixture['frames'])//3,2*len(fixture['frames'])//3,len(fixture['frames'])-1}
    seen=set()
    for frame,row in enumerate(fixture['frames']):
        key=tuple(row['selected'][2:])
        if key not in seen:frames.add(frame);seen.add(key)
    for frame in sorted(frames):
        reference=json.loads(subprocess.check_output([str(bevy.resolve()),'--fixture',str(fixture_path.resolve()),'--reference',str(frame)],text=True,timeout=30))
        canonical=json.loads(subprocess.check_output([str(theia.resolve()),'--benchmark-reference',str(fixture['players']),str(frame%period),str(period),*[str(v) for v in fixture['extent']]],text=True,timeout=30))
        expected=[o for o in canonical['objects'] if o['kind']==3]
        require(len(reference['objects'])==len(expected)==fixture['players'],'Wrong proxy population')
        require(reference['rows']==canonical['rows'] and reference['eye']==canonical['eye'],'Camera oracle differs')
        for obj,source in zip(reference['objects'],expected):
            require(obj['id']==source['id'] and all(struct.pack('<f',a)==struct.pack('<f',b) for a,b in zip(obj['extent'],source['extent'])),'Dense identity/extents differ')
            delta=max(abs(a-b) for a,b in zip(obj['center'][:2],source['center'][:2]));max_motion=max(max_motion,delta)
            require(delta<1e-6,'Proxy XY motion differs')
            region=int(obj['center'][0]>=0)+2*int(obj['center'][1]>=0);selected=reference['selected'][region];levels.add('fine' if selected<4 else 'coarse')
            origin,cells=surfaces[selected];cell=[min(7,int(math.floor((obj['center'][a]-origin[a])/8))) for a in range(2)]
            wanted=height(cells[cell[1]*8+cell[0]],obj['center'][:2])
            error=abs(obj['center'][2]-obj['extent'][2]-wanted);max_height=max(max_height,error)
            require(error<3e-6,'Bevy selected-terrain height differs from source')
            near=sum((a-b)**2 for a,b in zip(obj['center'],reference['eye']))<35**2
            require(obj['near']==near,'Proxy detail threshold differs');details.add('near' if near else 'far');count+=1
    result=dict(passed=True,poses=count,max_xy_error=max_motion,max_ground_error=max_height,terrain_levels=sorted(levels),detail_levels=sorted(details))
    output.write_text(json.dumps(result,indent=2)+'\n');return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--bevy',type=Path,default=Path('tools/theia-bevy-benchmark/target/release/theia-bevy-benchmark'))
    parser.add_argument('--theia',type=Path,default=Path('apps/theia-render-lab/target/release/theia-render-lab'))
    args=parser.parse_args();print(verify(args.fixture,args.bevy,args.theia,args.output))
