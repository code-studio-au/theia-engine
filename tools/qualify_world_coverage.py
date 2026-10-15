#!/usr/bin/env python3
"""CPU protected fallback/committed fine demand with real complete cooked graphs."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess


def require(value,message):
    if not value:raise RuntimeError(message)


def qualify(args):
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    result=out/'report.json';result.write_text('{"passed":false}\n')
    scene=json.loads(args.scene.read_bytes())
    require(scene['passed'] and len(scene['quadrants'])==len(scene['fallbacks'])==4,'Incomplete fine/fallback fixture')
    resources={};bundles=[];bounds=[]
    for group in ('fallbacks','quadrants'):
        for i,entry in enumerate(scene[group]):
            cook=entry['cook'];bundles.append(cook['bundle']['path'])
            owned={}
            for record in (cook['bundle'],cook,cook['materials'],cook['uv'],cook['texture_table'],*cook['textures']):
                blob=Path(record['path']).read_bytes();digest=hashlib.sha256(blob).hexdigest()
                require(digest==record['content_id'] and len(blob)==record['bytes'],'Source resource differs')
                owned[digest]=len(blob)
            resources[i+(4 if group=='fallbacks' else 0)]=owned
            if group=='quadrants':
                blob=Path(cook['path']).read_bytes();metadata,count=struct.unpack_from('<2I',blob,12)
                offset=76+((metadata+3)&~3)
                points=list(struct.iter_unpack('<3f',blob[offset:offset+count*12]))
                bounds.append(([min(p[a] for p in points) for a in (0,1)],[max(p[a] for p in points) for a in (0,1)]))
    def charge(ids):return sum({k:v for i in ids for k,v in resources[i].items()}.values())
    coarse=set(range(4,8));full=charge(range(8));base=charge(coarse)
    one=max(charge(coarse|{i}) for i in range(4))
    points=[[0.,0.]]
    for frame in range(240):
        raw=subprocess.check_output([str(args.renderer.resolve()),'--benchmark-reference','0',str(frame),'240','1280','720'],text=True,timeout=10)
        points.append(json.loads(raw)['eye'][:2])
    points += [[float(x),0.] for x in range(-110,111)]
    points += [[float(x),0.] for x in range(110,-111,-1)]
    points += [[-100.,-100.],[0.,0.],[100.,100.],[1000.,1000.]]
    points=[[struct.unpack('<f',struct.pack('<f',v))[0] for v in p] for p in points]
    route=out/'route.txt';route.write_text(''.join(' '.join(format(v,'.9g') for v in p)+'\n' for p in points))
    cases=[]
    for name,budget,slots,cancel in [('full',full,4,0),('one-fine',one,1,0),
            ('coarse-only',base,2,0),('short-full',full-1,4,0),('cancel-pending',full,4,3)]:
        command=[str(args.example.resolve()),'--covered-route','20','35',str(slots),str(budget),str(cancel),str(route),*bundles]
        process=subprocess.run(command,capture_output=True,text=True,check=True,timeout=30)
        (out/(name+'.jsonl')).write_text(process.stdout)
        rows=[json.loads(line) for line in process.stdout.splitlines()]
        require(len(rows)==len(points),'Lost covered route samples')
        actual=coarse.copy();epoch=1;rejections=cancellations=publications=0
        for frame,(eye,row) in enumerate(zip(points,rows)):
            eligible=[]
            for i,(lo,hi) in enumerate(bounds):
                closest=[min(hi[a],max(lo[a],eye[a])) for a in (0,1)]
                distance=sum((eye[a]-closest[a])**2 for a in (0,1))
                if distance<=(35 if i in actual else 20)**2:eligible.append((distance,i))
            eligible.sort();desired=[i for _,i in eligible[:slots]]
            loads=[i for i in desired if i not in actual];removals=sorted((actual-coarse)-set(desired))
            published=[];rejected=[];cancelled=[]
            for i in loads:
                if charge(actual|set(published)|{i})>budget:rejected.append(i)
                elif cancel and frame%cancel==0:cancelled.append(i)
                else:published.append(i)
            if published or removals:epoch+=1;publications+=1
            actual=(actual-set(removals))|set(published)
            expected=dict(frame=frame,desired=desired,loads=loads,removals=removals,
                deferred=[i for _,i in eligible[slots:]],published=published,rejected=rejected,cancelled=cancelled,
                resident=sorted(actual),selected=[i if i in actual else i+4 for i in range(4)],
                epoch=epoch,live_bytes=charge(actual))
            require(row==expected,'%s frame %d coverage/epoch/charge differs'%(name,frame))
            require(coarse<=actual and len(row['selected'])==4,'Fallback owner/selection missing')
            rejections+=len(rejected);cancellations+=len(cancelled)
        require(actual==coarse,'Final excursion failed to return to coarse coverage')
        if name in ('one-fine','coarse-only','short-full'):require(rejections>0,'Budget gate unexercised')
        if cancel:require(cancellations>0,'Pending cancellation unexercised')
        if name=='coarse-only':require(publications==0,'Coarse-only budget published fine content')
        cases.append(dict(case=name,budget=budget,samples=len(rows),transactions=publications,
            rejected_loads=rejections,cancelled_reservations=cancellations))
        print('Verified protected CPU coverage:',cases[-1],flush=True)
    report=dict(passed=True,cases=cases,coarse_encoded_bytes=base,all_encoded_bytes=full,
        samples_per_case=len(points),gpu_publication=False,runtime_streaming=False,
        scene_sha256=hashlib.sha256(args.scene.read_bytes()).hexdigest())
    result.write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--renderer',type=Path,default=Path('apps/theia-render-lab/target/debug/theia-render-lab'))
    parser.add_argument('--example',type=Path,default=Path('crates/theia-assets/target/debug/examples/bundle_residency'))
    qualify(parser.parse_args())
