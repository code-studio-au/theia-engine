// Shared-vertex mesh output for the specifically qualified GLSL fragment fallback.
#include "asset_camera.slang"
struct SharedVertex { float4 position : SV_Position; };
struct PrimitiveIdentity { nointerpolation uint2 identity : TEXCOORD0; };
[shader("mesh")]
[numthreads(64,1,1)]
[outputtopology("triangle")]
void assetVisibilityMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID, in payload AssetPayload payload,
    out vertices SharedVertex vertices[64], out indices uint3 triangles[64], out primitives PrimitiveIdentity primitives[64]) {
    uint positionOffset=76+((cookedMesh.Load(12)+3)&~3u);
    uint meshletOffset=positionOffset+cookedMesh.Load(16)*12;
    uint refsOffset=meshletOffset+cookedMesh.Load(20)*16;
    uint trianglesOffset=refsOffset+cookedMesh.Load(24)*4;
    uint meshletID=assetWork.Load(64+group.x*4);
    uint4 meshlet=cookedMesh.Load4(meshletOffset+meshletID*16);
    SetMeshOutputCounts(meshlet.z,meshlet.w);
    if (lane<meshlet.z) {
        uint global=cookedMesh.Load(refsOffset+(meshlet.x+lane)*4);
        float4 world=float4(asfloat(cookedMesh.Load3(positionOffset+global*12)),1);
        vertices[lane].position=float4(dot(camera.row0,world),dot(camera.row1,world),dot(camera.row2,world),dot(camera.row3,world));
    }
    if (lane<meshlet.w) {
        uint offset=trianglesOffset+meshlet.y+lane*3;
        triangles[lane]=uint3(byteAt(offset),byteAt(offset+1),byteAt(offset+2));
        primitives[lane].identity=uint2(meshletID+1,lane);
    }
}
