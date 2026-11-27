#!/usr/bin/env python3
"""Independent CPU selection/byte-union oracle for all fixture residency/request masks."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from qualify_world_retirement import resources
from world_camera_reference import require


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--scene',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--example',type=Path,default=Path('crates/theia-assets/target/debug/examples/bundle_residency'))
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    (out/'report.json').unlink(missing_ok=True)
    scene=json.loads(args.scene.read_bytes())
    require(scene['passed'] and len(scene['fallbacks'])==len(scene['quadrants'])==4,'Incomplete fixture')
    sets={i+offset:resources(entry['cook']) for group,offset in [('fallbacks',4),('quadrants',0)] for i,entry in enumerate(scene[group])}
    paths=[entry['cook']['bundle']['path'] for group in ('fallbacks','quadrants') for entry in scene[group]]
    process=subprocess.run([str(args.example.resolve()),'--covered-views',*paths],capture_output=True,text=True,check=True,timeout=30)
    (out/'views.jsonl').write_text(process.stdout);(out/'run.log').write_text(process.stderr)
    rows=[json.loads(line) for line in process.stdout.splitlines()]
    require(len(rows)==256,'Lost residency/request pairs')
    counts={'fine':0,'coarse_requested':0,'fine_unavailable':0}
    for index,row in enumerate(rows):
        resident=index//16;requested=index%16
        owned=set(range(4,8))|{i for i in range(4) if resident&(1<<i)}
        charge=sum({key:value for i in owned for key,value in sets[i].items()}.values())
        chosen=[i if resident&requested&(1<<i) else i+4 for i in range(4)]
        missing=[i for i in range(4) if requested&(1<<i) and not resident&(1<<i)]
        expected=dict(resident_mask=resident,requested_mask=requested,selected=chosen,refinement_needed=missing,
            epoch=1+int(resident!=0),live_bytes=charge)
        require(row==expected,'Selection/epoch/byte charge differs at pair '+str(index))
        for i in range(4):
            counts['fine' if chosen[i]<4 else 'fine_unavailable' if i in missing else 'coarse_requested']+=1
    require(counts==dict(fine=256,coarse_requested=512,fine_unavailable=256),'Selection branches unexercised')
    (out/'report.json').write_text(json.dumps(dict(passed=True,pairs=256,region_selections=1024,branches=counts,
        retained_epoch_release_checked=True,gpu_selection=False,camera_policy=False,
        scene_sha256=hashlib.sha256(args.scene.read_bytes()).hexdigest()),indent=2)+'\n')
    print('Verified all CPU view selections:',counts,flush=True)


if __name__=='__main__':main()
