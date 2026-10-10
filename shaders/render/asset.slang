#include "triangle.slang"
[[vk::binding(0,0)]] RWByteAddressBuffer assetWork;
[[vk::binding(1,0)]] ByteAddressBuffer cookedMesh;
struct AssetInput { uint scenario; };
[[vk::push_constant]] ConstantBuffer<AssetInput> assetInput;
[numthreads(1,1,1)]
void assetCompute() {
    uint count=cookedMesh.Load(20);
    if (assetInput.scenario==0) count=0;
    else if (assetInput.scenario==1) count=min(count,1);
    uint draw=count!=0 ? 1 : 0;
    assetWork.Store3(0,uint3(draw,1,1)); assetWork.Store(12,draw); assetWork.Store(36,count);
}
struct AssetPayload { uint count; };
groupshared AssetPayload assetPayload;
[shader("amplification")]
[numthreads(32,1,1)]
void assetTask(uint lane : SV_GroupIndex) {
    if (lane==0) assetPayload.count=assetWork.Load(36);
    GroupMemoryBarrierWithGroupSync();
    DispatchMesh(assetPayload.count,1,1,assetPayload);
}
uint byteAt(uint offset) { return (cookedMesh.Load(offset&~3u)>>((offset&3u)*8))&255u; }
[shader("mesh")]
[numthreads(64,1,1)]
[outputtopology("triangle")]
void assetMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID, in payload AssetPayload payload,
    out vertices Vertex vertices[64], out indices uint3 triangles[64]) {
    uint metadata=cookedMesh.Load(12);
    uint positionOffset=76+((metadata+3)&~3u);
    uint meshletOffset=positionOffset+cookedMesh.Load(16)*12;
    uint refsOffset=meshletOffset+cookedMesh.Load(20)*16;
    uint trianglesOffset=refsOffset+cookedMesh.Load(24)*4;
    uint4 meshlet=cookedMesh.Load4(meshletOffset+group.x*16);
    SetMeshOutputCounts(meshlet.z,meshlet.w);
    if (lane<meshlet.z) {
        uint vertex=cookedMesh.Load(refsOffset+(meshlet.x+lane)*4);
        float3 position=asfloat(cookedMesh.Load3(positionOffset+vertex*12));
        vertices[lane].position=float4(position,1);
        vertices[lane].color=float3(position.xy*0.5+0.5,0.5);
    }
    if (lane<meshlet.w) {
        uint offset=trianglesOffset+meshlet.y+lane*3;
        triangles[lane]=uint3(byteAt(offset),byteAt(offset+1),byteAt(offset+2));
    }
}
