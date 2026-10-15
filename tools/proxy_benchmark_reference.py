"""Pure offline procedural proxy/raster oracle shared by benchmark replay checks."""
import math


def dot(a,b):return sum(x*y for x,y in zip(a,b))
def sub(a,b):return [x-y for x,y in zip(a,b)]
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]

def pixel_reference(rows,objects,x,y,width,height):
    """Independent world-space ray/triangle oracle, including near/far rejection."""
    basis=[rows[i][:3] for i in (0,1,3)]
    eye=[sum(-rows[i][3]*rows[i][axis]/dot(rows[i][:3],rows[i][:3]) for i in (0,1,3)) for axis in range(3)]
    ndc=[(x+.5)/width*2-1,(y+.5)/height*2-1]
    direction=[basis[2][axis]+sum(ndc[i]*basis[i][axis]/dot(basis[i],basis[i]) for i in (0,1)) for axis in range(3)]
    closest=float('inf');result=(0,0,1.)
    cube=[(0,2,1),(1,2,3),(4,5,6),(5,7,6),(0,1,4),(1,5,4),(2,6,3),(3,6,7),(0,4,2),(2,4,6),(1,3,5),(3,7,5)]
    for id,center,extent,near in objects:
        lower=[center[a]-extent[a] for a in range(3)];upper=[center[a]+extent[a]*(2 if a==2 else 1) for a in range(3)]
        entry=0.;exit=float('inf')
        for axis in range(3):
            if abs(direction[axis])<1e-12:
                if not lower[axis]<=eye[axis]<=upper[axis]:exit=-1;break
            else:
                a=(lower[axis]-eye[axis])/direction[axis];b=(upper[axis]-eye[axis])/direction[axis]
                entry=max(entry,min(a,b));exit=min(exit,max(a,b))
        if entry>exit or entry>closest:continue
        vertices=[[center[a]+extent[a]*(1 if index&(1<<a) else -1) for a in range(3)] for index in range(8)]
        triangles=[[vertices[i] for i in tri] for tri in cube]
        if id<256:
            ground=[]
            for corner in range(4):
                px=center[0]+extent[0]*(1 if corner&1 else -1);py=center[1]+extent[1]*(1 if corner&2 else -1)
                ground.append([px,py,math.sin(px*.035)*math.cos(py*.035)*1.5])
            triangles=[[ground[i] for i in triangle] for triangle in [(0,1,2),(1,3,2)]]
        if near:
            roof=[[center[0]+dx*extent[0],center[1]+dy*extent[1],center[2]+extent[2]] for dx,dy in [(-1,-1),(1,-1),(1,1),(-1,1)]]
            peak=[center[0],center[1],center[2]+extent[2]*2]
            triangles.extend([[roof[i],roof[(i+1)%4],peak] for i in range(4)])
        for primitive,(a,b,c) in enumerate(triangles):
            edge1=sub(b,a);edge2=sub(c,a);p=cross(direction,edge2);det=dot(edge1,p)
            if abs(det)<1e-12:continue
            tvec=sub(eye,a);u=dot(tvec,p)/det
            if not 0<=u<=1:continue
            q=cross(tvec,edge1);v=dot(direction,q)/det
            if v<0 or u+v>1:continue
            t=dot(edge2,q)/det
            if not 0<t<closest:continue
            point=[eye[a]+direction[a]*t for a in range(3)]
            clip=[dot(row[:3],point)+row[3] for row in rows];depth=clip[2]/clip[3]
            if not 0<=depth<1:continue
            closest=t;result=(id+1,primitive,depth)
    return result

def color_reference(id,primitive,depth,kind):
    if not id:return [0.12,0.23,0.35]
    color=[[0.12,.3,.08],[.55,.38,.22],[.06,.23,.06],[.15,.3,.8]][kind]
    if kind==3:
        fraction=(id-1)%5/4;color=[a*(1-fraction)+b*fraction for a,b in zip(color,[.8,.15,.05])]
    if primitive>=12 and kind==1:color=[.3,.12,.06]
    light=.85 if kind==0 else [ .35,1,.55,.8,.5,.75][primitive//2] if primitive<12 else .7
    fog=min(.7,max(0,25/max(.1,250-depth*249.9)/180))
    return [a*light*(1-fog)+b*fog for a,b in zip(color,[.12,.23,.35])]

def encode(value):return round((value*12.92 if value<=.0031308 else 1.055*value**(1/2.4)-.055)*255)

def conservative_visible(rows,center,extent):
    common=63
    for corner in range(8):
        point=[center[a]+extent[a]*(1.5 if a==2 else 1)*(1 if corner&(1<<a) else -1)+(extent[2]*.5 if a==2 else 0) for a in range(3)]
        x,y,z,w=[dot(row[:3],point)+row[3] for row in rows]
        planes=int(x< -w)|(int(x>w)<<1)|(int(y< -w)<<2)|(int(y>w)<<3)|(int(z<0)<<4)|(int(z>w)<<5)
        common&=planes
    return common==0

