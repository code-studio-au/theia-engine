#!/usr/bin/env python3
"""Check native Bevy pixels against independent cooked/proxy triangle and colour oracles."""
import argparse
import json
import math
from pathlib import Path
import subprocess

from cooked_benchmark_reference import decode, material_factors, material_pixels, ray
from proxy_benchmark_reference import pixel_reference, color_reference, encode
from world_camera_reference import require


def verify(fixture_path,output,exe):
    from PIL import Image
    fixture=json.loads(fixture_path.read_bytes());meta=json.loads((output/'screenshot.json').read_bytes())
    width,height=meta['width'],meta['height'];frame=meta['frame'];selected=fixture['frames'][frame]['selected']
    require([width,height]==fixture['extent'] and meta['format'] in ('Bgra8UnormSrgb','Rgba8UnormSrgb'),'Unexpected screenshot format/extent')
    raw=(output/'screenshot.raw').read_bytes();require(len(raw)==width*height*4,'Screenshot is truncated/padded')
    image=Image.frombytes('RGBA',(width,height),raw,'raw','BGRA' if meta['format'].startswith('Bgra') else 'RGBA')
    image.save(output/'screenshot.png')
    reference=json.loads(subprocess.check_output([str(exe.resolve()),'--fixture',str(fixture_path.resolve()),'--reference',str(frame)],text=True,timeout=30))
    rows=reference['rows'];objects=[(o['id'],o['center'],o['extent'],o['near']) for o in reference['objects']]
    scene=json.loads(Path(fixture['source_manifest']).read_bytes());sources=[]
    for region,ident in enumerate(selected):
        cook=scene['quadrants' if ident<4 else 'fallbacks'][region]['cook']
        _,triangles,_,pixels=decode(cook);sources.append((triangles,material_factors(cook),material_pixels(cook,pixels)))
    coordinates={(int((x+.37)*width/15),int((y+.41)*height/9)) for y in range(9) for x in range(15)}
    # Probe projected proxy centers as well as a scene-wide grid. Nearest-hit
    # tests still include all world/proxy triangles, including occluders.
    for obj in reference['objects'][::max(1,len(objects)//50)]:
        clip=[sum(row[a]*obj['center'][a] for a in range(3))+row[3] for row in rows]
        if clip[3]<=0:continue
        x=int((clip[0]/clip[3]+1)*width/2);y=int((clip[1]/clip[3]+1)*height/2)
        if 0<=x<width and 0<=y<height:coordinates.add((x,y))
    counts=dict(world=0,proxy=0,background=0,texture_boundary_skips=0);failures=[];max_error=0
    def srgb(v):
        v/=255.;return v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4
    for x,y in sorted(coordinates):
        world=(0,0,1.,None,None);winner=None
        for source in sources:
            hit=ray(rows,source[0],x,y,width,height)
            if hit[0] and hit[2]<world[2]:world=hit;winner=source
        proxy=pixel_reference(rows,objects,x,y,width,height)
        if proxy[0] and proxy[2]<world[2]:
            linear=color_reference(*proxy,3);kind='proxy'
        elif world[0]:
            _,_,_,material,uv=world;factor=winner[1][material];texels=winner[2][material];side=int((len(texels)//4)**.5)
            coord=[(v-math.floor(v))*side for v in uv]
            if any(abs(v-round(v))<1e-4 for v in coord):counts['texture_boundary_skips']+=1;continue
            offset=(int(coord[1])*side+int(coord[0]))*4
            linear=[srgb(texels[offset+a])*factor[a] for a in range(3)];kind='world'
        else:linear=[.025,.07,.12];kind='background'
        expected=[encode(v) for v in linear];actual=list(image.getpixel((x,y))[:3]);error=max(abs(a-b) for a,b in zip(actual,expected));max_error=max(max_error,error)
        counts[kind]+=1
        if error>2:failures.append(dict(pixel=[x,y],kind=kind,expected=expected,actual=actual,error=error))
    result=dict(passed=not failures,frame=frame,players=fixture['players'],selected=selected,probes=counts,max_rgb_error=max_error,failures=failures)
    (output/'pixel-proof.json').write_text(json.dumps(result,indent=2)+'\n')
    require(not failures,'Bevy colour/source ray mismatch: '+str(failures[:3]))
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--exe',type=Path,default=Path('tools/theia-bevy-benchmark/target/release/theia-bevy-benchmark'))
    args=parser.parse_args();print(verify(args.fixture,args.output,args.exe))
