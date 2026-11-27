"""Independent replay checks for GPU proxies on selected cooked terrain."""
from collections import Counter
import math
import struct
import subprocess
import json
from qualify_world_ground import triangles, height as source_height
from proxy_benchmark_reference import conservative_visible, dot, sub, pixel_reference, color_reference, encode


def require(value,message):
    if not value:raise RuntimeError(message)


def table_reference(blob,scene):
    require(len(blob)==40704,'Unexpected bounded ground table size')
    end=256
    for index in range(8):
        x,y,vertices,offsets,refs,count,length,reserved=struct.unpack_from('<2f6I',blob,index*32)
        slot=index%4
        require((x,y)==((slot%2-1)*64,(slot//2-1)*64) and reserved==0 and vertices==end,'Ground header differs')
        require(offsets==vertices+count*48 and refs==offsets+65*4,'Ground table ranges differ')
        actual=[]
        for i in range(count):
            points=[]
            for corner in range(3):
                px,py,pz,pad=struct.unpack_from('<4f',blob,vertices+i*48+corner*16)
                require(pad==0,'Ground vertex padding differs');points.append((px,py,pz))
            actual.append(tuple(points))
        expected=triangles(scene['quadrants' if index<4 else 'fallbacks'][slot])
        require(Counter(actual)==Counter(expected),'GPU ground triangles differ from source')
        pointers=struct.unpack_from('<65I',blob,offsets)
        require(pointers[0]==0 and pointers[-1]==length,'Ground CSR endpoints differ')
        for cell in range(64):
            lo=[x+8*(cell%8),y+8*(cell//8)]
            wanted={i for i,t in enumerate(actual) if all(max(p[axis] for p in t)>lo[axis] and min(p[axis] for p in t)<lo[axis]+8 for axis in (0,1))}
            start,stop=pointers[cell:cell+2]
            require(0<=start<stop<=length and stop-start<=8,'Ground CSR range differs')
            ids=struct.unpack_from('<%dI'%(stop-start),blob,refs+start*4)
            require(len(ids)==len(set(ids)) and set(ids)==wanted,'Ground CSR candidate coverage differs')
        end=refs+length*4
    require(end==len(blob),'Ground table trailing bytes differ')


def verify(controller,buffers,scene,selected,exe,players,frame,period,width,height):
    import renderdoc as rd
    work=[b for b in buffers if b.length==376160 and b.creationFlags&rd.BufferCategory.ReadWrite];tables=[b for b in buffers if b.length==40704 and b.creationFlags&rd.BufferCategory.ReadWrite]
    require(len(work)==len(tables)==1,'Missing proxy work/ground table')
    table_reference(bytes(controller.GetBufferData(tables[0].resourceId,0,40704)),scene)
    dynamic=controller.GetBufferData(work[0].resourceId,0,376160)
    count,y,z,draws,total,near,far,visual=struct.unpack_from('<8I',dynamic)
    require((y,z,draws,total)==(1,1,1,players) and count<=players,'Proxy work header differs')
    reference=json.loads(subprocess.check_output([str(exe),'--benchmark-reference',str(players),str(frame%period),str(period),str(width),str(height)]))
    sources=[triangles(scene['quadrants' if value<4 else 'fallbacks'][i]) for i,value in enumerate(selected)]
    expected_visible=[];objects={};max_motion=0.;max_ground=0.
    for obj in reference['objects']:
        if obj['kind']!=3:continue
        ident=obj['id'];record=struct.unpack_from('<3fI3fI',dynamic,41824+ident*32)
        require(all(math.isfinite(v) for v in record[:3]),'Failed GPU terrain query/nonfinite proxy')
        motion=max(abs(a-b) for a,b in zip(record[:2],obj['center'][:2]));max_motion=max(max_motion,motion)
        slot=int(record[0]>=0)+2*int(record[1]>=0)
        ground_error=abs(record[2]-record[6]-source_height(sources[slot],record[:2]));max_ground=max(max_ground,ground_error)
        require(motion<=.001 and ground_error<3e-6 and record[3]==0x80000003 and all(abs(a-b)<1e-6 for a,b in zip(record[4:7],obj['extent'])),'Proxy motion/selected-ground/extent differs')
        is_near=dot(sub(record[:3],reference['eye']),sub(record[:3],reference['eye']))<35*35
        require(bool(record[7])==is_near,'Proxy near/detail rule differs')
        objects[ident]=(ident,record[:3],record[4:7],is_near)
        if conservative_visible(reference['rows'],record[:3],record[4:7]):expected_visible.append(ident)
    ids=list(struct.unpack_from('<%dI'%count,dynamic,32))
    require(len(ids)==len(set(ids)) and sorted(ids)==expected_visible,'Proxy GPU compaction differs')
    require((count,near,far,visual)==(len(ids),sum(objects[i][3] for i in ids),sum(not objects[i][3] for i in ids),len(ids)),'Proxy GPU counters differ')
    return dict(objects=objects,visible=set(ids),rays=[objects[i] for i in ids],
        proof=dict(players=players,visible=count,near=near,far=far,max_motion_error=max_motion,max_ground_error=max_ground,ground_table_verified=True))


def adjacent_depth_hit(rows,proxies,actual,stored_depth,expected,x,y,width,height):
    """Resolve only proxy/proxy ambiguity at the same or adjacent D32 code.

    The captured raster depth must exactly equal the independently intersected
    actual surface after D32 conversion. Primitive, geometry membership and colour
    remain checked; this never accepts a foreign ID, missed triangle or distant hit.
    """
    flag=0x80000000
    if not actual[0]&flag or not expected[0]&flag or actual==expected[:2]:return None
    ident=(actual[0]&~flag)-1
    if ident not in proxies['visible']:return None
    hit=pixel_reference(rows,[proxies['objects'][ident]],x,y,width,height)
    if (hit[0]|flag,hit[1])!=actual:return None
    def code(depth):return struct.unpack('<I',struct.pack('<f',depth))[0]
    nearest,candidate,stored=map(code,(expected[2],hit[2],stored_depth))
    if candidate-nearest not in (0,1) or stored!=candidate:return None
    return dict(identity=actual,depth=hit[2],ideal_nearest=expected[:2],nearest_depth_code=nearest,
        actual_source_depth_code=candidate,stored_depth_code=stored)
