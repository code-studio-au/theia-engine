#include "asset_camera.slang"
// Conservative homogeneous hull test: reject only if every referenced vertex
// lies outside one common clip plane. Surviving meshlets may still rasterize nothing.
groupshared uint culledVisibleCount;
groupshared uint culledVisibleMask;
[numthreads(32,1,1)]
void assetCullCompute(uint lane : SV_GroupIndex) {
    if (lane==0) { culledVisibleCount=0; culledVisibleMask=0; }
    GroupMemoryBarrierWithGroupSync();
    if (lane<cookedMesh.Load(20)) {
        uint positionOffset=76+((cookedMesh.Load(12)+3)&~3u);
        uint meshletOffset=positionOffset+cookedMesh.Load(16)*12;
        uint refsOffset=meshletOffset+cookedMesh.Load(20)*16;
        uint4 meshlet=cookedMesh.Load4(meshletOffset+lane*16);
        uint commonPlanes=63;
        for (uint i=0; i<meshlet.z; ++i) {
            uint vertex=cookedMesh.Load(refsOffset+(meshlet.x+i)*4);
            float4 position=float4(asfloat(cookedMesh.Load3(positionOffset+vertex*12)),1);
            float4 clip=float4(dot(camera.row0,position),dot(camera.row1,position),dot(camera.row2,position),dot(camera.row3,position));
            uint planes=(clip.x < -clip.w ? 1u:0u) | (clip.x>clip.w ? 2u:0u)
                | (clip.y < -clip.w ? 4u:0u) | (clip.y>clip.w ? 8u:0u)
                | (clip.z<0 ? 16u:0u) | (clip.z>clip.w ? 32u:0u);
            commonPlanes &= planes;
        }
        if (commonPlanes==0) {
            uint output; InterlockedAdd(culledVisibleCount,1,output);
            InterlockedOr(culledVisibleMask,1u<<lane);
            assetWork.Store(64+output*4,lane);
        }
    }
    AllMemoryBarrierWithGroupSync();
    if (lane==0) {
        uint draw=culledVisibleCount!=0 ? 1u:0u;
        assetWork.Store3(0,uint3(draw,1,1)); assetWork.Store(12,draw);
        assetWork.Store(36,culledVisibleCount); assetWork.Store(40,culledVisibleMask);
    }
}
[shader("mesh")]
[numthreads(64,1,1)]
[outputtopology("triangle")]
void assetCulledMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID, in payload AssetPayload payload,
    out vertices Vertex vertices[64], out indices uint3 triangles[64]) {
    uint metadata=cookedMesh.Load(12);
    uint positionOffset=76+((metadata+3)&~3u);
    uint meshletOffset=positionOffset+cookedMesh.Load(16)*12;
    uint refsOffset=meshletOffset+cookedMesh.Load(20)*16;
    uint trianglesOffset=refsOffset+cookedMesh.Load(24)*4;
    uint4 meshlet=cookedMesh.Load4(meshletOffset+assetWork.Load(64+group.x*4)*16);
    SetMeshOutputCounts(meshlet.z,meshlet.w);
    if (lane<meshlet.z) {
        uint vertex=cookedMesh.Load(refsOffset+(meshlet.x+lane)*4);
        float3 position=asfloat(cookedMesh.Load3(positionOffset+vertex*12));
        float4 world=float4(position,1);
        vertices[lane].position=float4(dot(camera.row0,world),dot(camera.row1,world),dot(camera.row2,world),dot(camera.row3,world));
        vertices[lane].color=float3(position.xy*0.5+0.5,0.5);
    }
    if (lane<meshlet.w) {
        uint offset=trianglesOffset+meshlet.y+lane*3;
        triangles[lane]=uint3(byteAt(offset),byteAt(offset+1),byteAt(offset+2));
    }
}
