#include "triangle.slang"
[[vk::binding(0,0)]] RWByteAddressBuffer lodWork;
[[vk::binding(1,0)]] ByteAddressBuffer fineMesh;
[[vk::binding(2,0)]] ByteAddressBuffer coarseMesh;
struct LodInput {
    float4 row0; float4 row1; float4 row2; float4 row3;
#ifndef THEIA_LOD_MESH
    float4 boundsMin; // xyz common bounds, w upward represented vertical error
    float4 boundsMax; // xyz common bounds, w pixel budget
    float4 viewport; // xy actual extent, zw reserved
#endif
};
[[vk::push_constant]] ConstantBuffer<LodInput> lod;
uint lodLoad(uint offset) {
    if (lodWork.Load(44)==1) return coarseMesh.Load(offset);
    return fineMesh.Load(offset);
}
uint3 lodLoad3(uint offset) { return uint3(lodLoad(offset),lodLoad(offset+4),lodLoad(offset+8)); }
uint4 lodLoad4(uint offset) { return uint4(lodLoad(offset),lodLoad(offset+4),lodLoad(offset+8),lodLoad(offset+12)); }
uint lodByte(uint offset) { return (lodLoad(offset&~3u)>>((offset&3u)*8))&255u; }
float4 lodClip(float4 p) { return float4(dot(lod.row0,p),dot(lod.row1,p),dot(lod.row2,p),dot(lod.row3,p)); }
#ifndef THEIA_LOD_MESH
groupshared uint lodVisibleCount;
groupshared uint lodVisibleMask;
[numthreads(32,1,1)]
void assetLodCompute(uint lane:SV_GroupIndex) {
    if (lane==0) {
        lodVisibleCount=0; lodVisibleMask=0;
        uint selected=0; uint reason=1;
        float2 pixels=float2(0,0); float minimumW=1e30;
        float2 maximumXY=float2(0,0);
        // Bounded f32 qualification policy. Absolute/relative margins absorb
        // arithmetic/FTZ differences; near-boundary or unsupported inputs keep fine.
        bool zero=(asuint(lod.boundsMin.w)&0x7fffffffu)==0;
        bool qualified=all(abs(lod.row0)<=64) && all(abs(lod.row1)<=64)
            && all(abs(lod.row2)<=64) && all(abs(lod.row3)<=64)
            && all(abs(lod.boundsMin.xyz)<=64) && all(abs(lod.boundsMax.xyz)<=64)
            && (zero || (lod.boundsMin.w>=1e-6 && lod.boundsMin.w<=1))
            && all(lod.viewport.xy>=1) && all(lod.viewport.xy<=16384);
        bool clipped=false;
        for(uint corner=0;corner<8;++corner) {
            float4 p=float4((corner&1)!=0 ? lod.boundsMax.x:lod.boundsMin.x,
                (corner&2)!=0 ? lod.boundsMax.y:lod.boundsMin.y,
                (corner&4)!=0 ? lod.boundsMax.z:lod.boundsMin.z,1);
            float4 clip=lodClip(p);
            float4 magnitude=float4(dot(abs(lod.row0),abs(p)),dot(abs(lod.row1),abs(p)),
                dot(abs(lod.row2),abs(p)),dot(abs(lod.row3),abs(p)));
            float4 margin=magnitude*2e-6+1e-6;
            minimumW=min(minimumW,clip.w-margin.w);
            maximumXY=max(maximumXY,abs(clip.xy)+margin.xy);
            clipped |= clip.w-margin.w<=1e-4 || clip.z-margin.z<=1e-4
                || (clip.w-margin.w)-(clip.z+margin.z)<=1e-4;
        }
        if (!qualified) reason=3;
        else if (!clipped) {
            pixels=zero ? float2(0,0) : lod.boundsMin.w*
                (float2(abs(lod.row0.z),abs(lod.row1.z))+maximumXY*abs(lod.row3.z)/minimumW)
                /minimumW*lod.viewport.xy*.5;
            if (!zero) pixels=pixels*1.0001+.001;
            selected=all(pixels<=lod.boundsMax.w) ? 1u:0u;
            reason=selected==1 ? 0u:2u;
        }
        lodWork.Store(44,selected); lodWork.Store2(48,asuint(pixels));
        lodWork.Store(56,reason); lodWork.Store(60,asuint(minimumW));
    }
    AllMemoryBarrierWithGroupSync();
    uint count=lodLoad(20);
    if (lane<count) {
        uint positions=76+((lodLoad(12)+3)&~3u);
        uint meshlets=positions+lodLoad(16)*12;
        uint refs=meshlets+count*16;
        uint4 meshlet=lodLoad4(meshlets+lane*16);
        uint common=63;
        for(uint i=0;i<meshlet.z;++i) {
            uint vertex=lodLoad(refs+(meshlet.x+i)*4);
            float4 clip=lodClip(float4(asfloat(lodLoad3(positions+vertex*12)),1));
            uint planes=(clip.x < -clip.w ? 1u:0u)|(clip.x>clip.w ? 2u:0u)
                |(clip.y < -clip.w ? 4u:0u)|(clip.y>clip.w ? 8u:0u)
                |(clip.z<0 ? 16u:0u)|(clip.z>clip.w ? 32u:0u);
            common &= planes;
        }
        if(common==0) {
            uint output;InterlockedAdd(lodVisibleCount,1,output);
            InterlockedOr(lodVisibleMask,1u<<lane);lodWork.Store(64+output*4,lane);
        }
    }
    AllMemoryBarrierWithGroupSync();
    if(lane==0) {
        uint draw=lodVisibleCount!=0 ? 1u:0u;
        lodWork.Store3(0,uint3(draw,1,1));lodWork.Store(12,draw);
        lodWork.Store(32,count);lodWork.Store(36,lodVisibleCount);lodWork.Store(40,lodVisibleMask);
    }
}
#endif
struct LodPayload {uint count;};
[shader("mesh")]
[numthreads(64,1,1)]
[outputtopology("triangle")]
void assetLodMesh(uint lane:SV_GroupIndex,uint3 group:SV_GroupID,in payload LodPayload payload,
    out vertices Vertex vertices[64],out indices uint3 triangles[64]) {
    uint positions=76+((lodLoad(12)+3)&~3u);
    uint meshlets=positions+lodLoad(16)*12;
    uint refs=meshlets+lodLoad(20)*16;
    uint triangleBytes=refs+lodLoad(24)*4;
    uint4 meshlet=lodLoad4(meshlets+lodWork.Load(64+group.x*4)*16);
    SetMeshOutputCounts(meshlet.z,meshlet.w);
    if(lane<meshlet.z) {
        uint vertex=lodLoad(refs+(meshlet.x+lane)*4);
        float3 position=asfloat(lodLoad3(positions+vertex*12));
        vertices[lane].position=lodClip(float4(position,1));
        vertices[lane].color=float3(position.xy*.5+.5,.5);
    }
    if(lane<meshlet.w) {
        uint offset=triangleBytes+meshlet.y+lane*3;
        triangles[lane]=uint3(lodByte(offset),lodByte(offset+1),lodByte(offset+2));
    }
}
