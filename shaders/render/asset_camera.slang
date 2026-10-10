#include "asset.slang"
struct CameraRows { float4 row0; float4 row1; float4 row2; float4 row3; };
[[vk::push_constant]] ConstantBuffer<CameraRows> camera;
[shader("mesh")]
[numthreads(64,1,1)]
[outputtopology("triangle")]
void assetCameraMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID, in payload AssetPayload payload,
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
        float4 world=float4(position,1);
        vertices[lane].position=float4(dot(camera.row0,world),dot(camera.row1,world),dot(camera.row2,world),dot(camera.row3,world));
        vertices[lane].color=float3(position.xy*0.5+0.5,0.5);
    }
    if (lane<meshlet.w) {
        uint offset=trianglesOffset+meshlet.y+lane*3;
        triangles[lane]=uint3(byteAt(offset),byteAt(offset+1),byteAt(offset+2));
    }
}
