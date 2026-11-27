#ifdef THEIA_WORLD_GROUND
#define THEIA_GROUND_BINDING 1
#include "world_ground_query.slang"
#endif
// Visual-only deterministic workload. Dense records; no gameplay authority.
static const uint STATIC_OBJECTS=448, MAX_OBJECTS=10448, INSTANCE_OFFSET=41824;
[[vk::binding(0,0)]] RWByteAddressBuffer benchmarkWork;
struct Parameters { float4 row0,row1,row2,row3; float4 eyeTime; uint4 counts; };
[[vk::push_constant]] ConstantBuffer<Parameters> bench;
float terrain(float2 p) { return sin(p.x*0.035)*cos(p.y*0.035)*1.5; }
float4 clip(float3 p) { float4 v=float4(p,1); return float4(dot(bench.row0,v),dot(bench.row1,v),dot(bench.row2,v),dot(bench.row3,v)); }
float coarseTerrain(float2 p) {
    float2 base=clamp(floor((p+64)/16),0,7)*16-64,uv=(p-base)/16;
    float a=terrain(base),b=terrain(base+float2(16,0)),c=terrain(base+float2(0,16)),d=terrain(base+16);
    return uv.x+uv.y<=1 ? a+uv.x*(b-a)+uv.y*(c-a):d+(1-uv.x)*(c-d)+(1-uv.y)*(b-d);
}
[shader("compute")]
[numthreads(64,1,1)]
void benchmarkCompute(uint3 thread:SV_DispatchThreadID) {
    bool mixed=bench.counts.z!=0;
    uint id=thread.x+(mixed ? STATIC_OBJECTS:0); if(id>=bench.counts.y) return;
    float2 p;float3 extent;uint kind;
    if(id<256) { p=float2(float(id%16)-7.5,float(id/16)-7.5)*8;extent=float3(4,4,0.5);kind=0; }
    else if(id<320) {uint i=id-256;p=float2(float(i%8)-3.5,float(i/8)-3.5)*12;extent=float3(2.5,2.5,3);kind=1;}
    else if(id<STATIC_OBJECTS) {uint i=id-320;float a=float((i*2654435761u)>>16)*(6.283185307179586/65536);p=float2(sin(a),cos(a))*55;extent=float3(0.7,0.7,2);kind=2;}
    // Integer phase hash keeps trigonometric arguments bounded for all 10000 IDs.
    else {uint i=id-STATIC_OBJECTS;float a=float((i*2654435761u)>>16)*(6.283185307179586/65536)+bench.eyeTime.w*(0.12+float(i%7)*0.01);float r=sqrt(float(i+1)/max(1.0,float(bench.counts.x)))*22;p=float2(cos(a)*r,25+sin(a)*r);extent=float3(0.3,0.3,0.8);kind=3;}
    float ground;
#ifdef THEIA_WORLD_GROUND
    uint4 query=queryGround(p,bench.counts.w);
    // A failed height query stays explicit in its record for replay qualification.
    ground=query.z==0 ? asfloat(query.x):asfloat(0x7fc00000u);
#else
    ground=mixed ? coarseTerrain(p):terrain(p);
#endif
    float3 center=float3(p,ground+(kind==0 ? 0:extent.z));
    bool near=kind!=0 && dot(center-bench.eyeTime.xyz,center-bench.eyeTime.xyz)<35*35;
    uint offset=INSTANCE_OFFSET+id*32;
    benchmarkWork.Store3(offset,asuint(center));benchmarkWork.Store(offset+12,kind|(mixed ? 0x80000000u:0u));
    benchmarkWork.Store3(offset+16,asuint(extent));benchmarkWork.Store(offset+28,near ? 1u:0u);
    uint common=63;
    for(uint c=0;c<8;++c) {
        float3 bounds=extent*float3(1,1,1.5);
        float3 corner=center+bounds*float3((c&1)!=0 ? 1:-1,(c&2)!=0 ? 1:-1,(c&4)!=0 ? 1:-1)+float3(0,0,extent.z*0.5);
        float4 v=clip(corner);
        common &= (v.x < -v.w ? 1u:0u)|(v.x>v.w ? 2u:0u)|(v.y < -v.w ? 4u:0u)|(v.y>v.w ? 8u:0u)|(v.z<0 ? 16u:0u)|(v.z>v.w ? 32u:0u);
    }
    if(common==0) {
        uint output;benchmarkWork.InterlockedAdd(0,1,output);benchmarkWork.Store(32+output*4,id);
        uint ignored;benchmarkWork.InterlockedAdd(near ? 20:24,1,ignored);
        if(kind==3) benchmarkWork.InterlockedAdd(28,1,ignored);
    }
}
struct Payload {uint id;};
groupshared Payload benchPayload;
[shader("amplification")]
[numthreads(32,1,1)]
void benchmarkTask(uint lane:SV_GroupIndex,uint3 group:SV_GroupID) {
    if(lane==0) benchPayload.id=benchmarkWork.Load(32+group.x*4);
    GroupMemoryBarrierWithGroupSync();DispatchMesh(1,1,1,benchPayload);
}
struct Vertex {float4 position:SV_Position; nointerpolation uint2 identity:TEXCOORD0;};
static const uint3 cubeTriangles[12]={uint3(0,2,1),uint3(1,2,3),uint3(4,5,6),uint3(5,7,6),uint3(0,1,4),uint3(1,5,4),uint3(2,6,3),uint3(3,6,7),uint3(0,4,2),uint3(2,4,6),uint3(1,3,5),uint3(3,7,5)};
float3 corner(uint c) { return float3((c&1)!=0 ? 1:-1,(c&2)!=0 ? 1:-1,(c&4)!=0 ? 1:-1); }
[shader("mesh")]
[numthreads(64,1,1)]
[outputtopology("triangle")]
void benchmarkMesh(uint lane:SV_GroupIndex,in payload Payload payload,out vertices Vertex vertices[48],out indices uint3 triangles[16]) {
    uint offset=INSTANCE_OFFSET+payload.id*32;
    float3 center=asfloat(benchmarkWork.Load3(offset)),extent=asfloat(benchmarkWork.Load3(offset+16));
    uint flags=benchmarkWork.Load(offset+12),kind=flags&3u;
    uint count=kind==0 ? 2:(benchmarkWork.Load(offset+28)!=0 ? 16:12);
    SetMeshOutputCounts(count*3,count);
    if(lane<count*3) {
        uint primitive=lane/3,vertex=lane%3;float3 p;
        if(kind==0) {
            uint3 quad[2]={uint3(0,1,2),uint3(1,3,2)};
            uint c=quad[primitive][vertex];
            p=float3(center.xy+extent.xy*float2((c&1)!=0 ? 1:-1,(c&2)!=0 ? 1:-1),0);
            // Adjacent cells evaluate identical world XY at each shared edge vertex.
            p.z=terrain(p.xy);
        }
        else if(primitive<12) p=center+extent*corner(cubeTriangles[primitive][vertex]);
        else {
            uint side=primitive-12;
            float2 base[4]={float2(-1,-1),float2(1,-1),float2(1,1),float2(-1,1)};
            p=center+float3(vertex==2 ? float2(0,0):base[(side+vertex)%4]*extent.xy,extent.z*(vertex==2 ? 2:1));
        }
        vertices[lane].position=clip(p);vertices[lane].identity=uint2((payload.id+1)|(flags&0x80000000u),primitive);
    }
    if(lane<count) triangles[lane]=uint3(lane*3,lane*3+1,lane*3+2);
}
[shader("fragment")]
uint2 benchmarkVisibility(Vertex input):SV_Target0 {return input.identity;}
