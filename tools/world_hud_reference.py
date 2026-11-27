"""Check streamed HUD glyphs against verified GPU work and completed timings."""
import math
from pathlib import Path
import re
from proxy_benchmark_reference import encode
from world_timing_reference import f32
from world_camera_reference import require


def verify(rgb,color,width,height,timings,frame,world_work,models,proxy):
    recent=timings['samples'][max(0,frame-30):frame]
    last=timings['samples'][max(0,frame-1)];sample=timings['samples'][frame];stats=sample['world']
    scaled=lambda value,scale:math.floor(value*scale+.5)
    interval=sum(s['frame_interval_ms'] for s in reversed(recent))
    fps=scaled(1000*len(recent)/interval,10) if recent else 0
    cpu,gpu=(scaled(last[key],1000) if frame else 0 for key in ('cpu_draw_ms','gpu_total_ms'))
    asset=scaled(stats['total_gpu_bytes']/1048576,1000)
    draw=scaled(sample['tracked_image_and_work_allocation_bytes']/1048576,10)
    clock=scaled(sample['scene_time'],10);phase=min(2,math.floor(f32(f32(sample['scene_time']/60)*3)))+1
    meshlets=sum(row[0] for row in world_work);total=sum(len(model[2]) for model in models)
    textures=sum(len(model[3]) for model in models)
    require(stats['selected_images']==textures,'HUD selected image count differs')
    lines=['THEIA STREAMED',f'FPS {fps/10:.1f}',f'CPU MS {cpu/1000:.3f}',f'GPU MS {gpu/1000:.3f}',
        f'MESHLETS {meshlets}/{total} TEX {textures}',f'ASSET MIB {asset/1000:.3f}',f'DRAW MIB {draw/10:.1f}',
        f'PUB {stats["publications"]} EVICT {stats["evictions"]}',f'PHASE {phase} T {clock/10:.1f} DEF {stats["deferrals"]}',
        f'PROXIES {proxy["visible"]}/{proxy["players"]}',f'NEAR {proxy["near"]} FAR {proxy["far"]}']
    source=Path('shaders/render/benchmark_hud.slang').read_text()
    font=[(int(a,16),int(b)) for a,b in re.findall(r'uint2\(0x([0-9a-f]+)u,(\d+)u\)',source)]
    require(len(font)==38,'Font encoding contract changed')
    chars='0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ./';checks=0
    for line,text in enumerate(lines):
        for column,char in enumerate(text):
            for row in range(7):
                for cell in range(5):
                    index=chars.find(char)
                    bits=(((font[index][0]>>(row*5))&31) if row<6 else font[index][1]) if index>=0 else 0
                    lit=bool(bits&(1<<(4-cell)))
                    x=24+column*18+cell*3+1;y=24+line*24+row*3+1
                    require(12<=x<min(480,width) and 12<=y<min(300,height),'HUD glyph is clipped')
                    actual=list(rgb[(y*width+x)*4:(y*width+x)*4+3])
                    if color.format.BGRAOrder():actual.reverse()
                    expected=[encode(v) for v in ([.7,.92,.76] if lit else [.012,.025,.04])]
                    require(all(abs(a-b)<=2 for a,b in zip(actual,expected)),'Streamed HUD glyph differs: '+text+' column '+str(column))
                    checks+=1
    return dict(overlay_verified=True,overlay_lines=lines,overlay_glyph_pixels=checks,gpu_counter_readback_for_sizing=False)
