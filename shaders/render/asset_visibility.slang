#include "asset_camera.slang"
// Bounded identity qualification: three vertices per triangle avoid an unqualified
// per-primitive fragment interface. Input meshlets still have <=64 vertices/triangles.
struct SceneVisibilityVertex { float4 position : SV_Position; nointerpolation uint2 identity : TEXCOORD0; };
[shader("mesh")]
[numthreads(64,1,1)]
[outputtopology("triangle")]
void assetVisibilityMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID, in payload AssetPayload payload,
    out vertices SceneVisibilityVertex vertices[192], out indices uint3 triangles[64]) {
    uint positionOffset=76+((cookedMesh.Load(12)+3)&~3u);
    uint meshletOffset=positionOffset+cookedMesh.Load(16)*12;
    uint refsOffset=meshletOffset+cookedMesh.Load(20)*16;
    uint trianglesOffset=refsOffset+cookedMesh.Load(24)*4;
    uint meshletID=assetWork.Load(64+group.x*4);
    uint4 meshlet=cookedMesh.Load4(meshletOffset+meshletID*16);
    SetMeshOutputCounts(meshlet.w*3,meshlet.w);
    if (lane<meshlet.w) {
        uint offset=trianglesOffset+meshlet.y+lane*3;
        for (uint axis=0; axis<3; ++axis) {
            uint local=byteAt(offset+axis);
            uint global=cookedMesh.Load(refsOffset+(meshlet.x+local)*4);
            float4 world=float4(asfloat(cookedMesh.Load3(positionOffset+global*12)),1);
            vertices[lane*3+axis].position=float4(dot(camera.row0,world),dot(camera.row1,world),dot(camera.row2,world),dot(camera.row3,world));
            vertices[lane*3+axis].identity=uint2(meshletID+1,lane);
        }
        triangles[lane]=uint3(lane*3,lane*3+1,lane*3+2);
    }
}
[shader("fragment")]
uint2 assetVisibilityFragment(SceneVisibilityVertex input) : SV_Target0 { return input.identity; }
