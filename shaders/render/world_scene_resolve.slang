#include "resolve.slang"
[[vk::binding(4,0)]] ByteAddressBuffer worldGeometry[4];
[[vk::binding(5,0)]] ByteAddressBuffer worldFactors[4];
[[vk::binding(6,0)]] ByteAddressBuffer worldUVs[4];
[[vk::binding(7,0)]] Texture2D<float4> residentTextures[];
[[vk::binding(8,0)]] SamplerState residentSampler;
[[vk::binding(9,0)]] ByteAddressBuffer worldTables[4];
struct SceneCamera { float4 row0; float4 row1; float4 row2; float4 row3; uint4 textureOffsets; };
[[vk::push_constant]] ConstantBuffer<SceneCamera> sceneCamera;
uint worldByte(uint chunk,uint offset) { return (worldGeometry[NonUniformResourceIndex(chunk)].Load(offset&~3u)>>((offset&3u)*8))&255u; }
bool reconstructWorldUV(float4 pixel, uint2 identity, uint chunk, out float2 result) {
    result=0;
    if (identity.x==0) return false;
    uint meshletID=identity.x-1;
    if (meshletID>=worldGeometry[NonUniformResourceIndex(chunk)].Load(20)) return false;
    uint positionOffset=76+((worldGeometry[NonUniformResourceIndex(chunk)].Load(12)+3)&~3u);
    uint meshletOffset=positionOffset+worldGeometry[NonUniformResourceIndex(chunk)].Load(16)*12;
    uint refsOffset=meshletOffset+worldGeometry[NonUniformResourceIndex(chunk)].Load(20)*16;
    uint trianglesOffset=refsOffset+worldGeometry[NonUniformResourceIndex(chunk)].Load(24)*4;
    uint4 meshlet=worldGeometry[NonUniformResourceIndex(chunk)].Load4(meshletOffset+meshletID*16);
    if (identity.y>=meshlet.w) return false;
    uint uvOffset=88+((worldUVs[NonUniformResourceIndex(chunk)].Load(12)+3)&~3u);
    float2 uv[3]; float4 clip[3]; float2 screen[3];
    for (uint axis=0; axis<3; ++axis) {
        uint local=worldByte(chunk,trianglesOffset+meshlet.y+identity.y*3+axis);
        uint global=worldGeometry[NonUniformResourceIndex(chunk)].Load(refsOffset+(meshlet.x+local)*4);
        if (global>=worldUVs[NonUniformResourceIndex(chunk)].Load(16)) return false;
        uv[axis]=asfloat(worldUVs[NonUniformResourceIndex(chunk)].Load2(uvOffset+global*8));
        float4 position=float4(asfloat(worldGeometry[NonUniformResourceIndex(chunk)].Load3(positionOffset+global*12)),1);
        clip[axis]=float4(dot(sceneCamera.row0,position),dot(sceneCamera.row1,position),dot(sceneCamera.row2,position),dot(sceneCamera.row3,position));
        screen[axis]=clip[axis].xy/clip[axis].w;
    }
    uint width,height;visibilityImage.GetDimensions(width,height);
    float2 point=pixel.xy/float2(width,height)*2-1;
    float2 a=screen[0],b=screen[1],c=screen[2];
    float denominator=(b.y-c.y)*(a.x-c.x)+(c.x-b.x)*(a.y-c.y);
    if (abs(denominator)<1e-10) return false;
    float u=((b.y-c.y)*(point.x-c.x)+(c.x-b.x)*(point.y-c.y))/denominator;
    float v=((c.y-a.y)*(point.x-c.x)+(a.x-c.x)*(point.y-c.y))/denominator;
    float3 weights=float3(u,v,1-u-v);
    float3 reciprocal=weights/float3(clip[0].w,clip[1].w,clip[2].w);
    result=(uv[0]*reciprocal.x+uv[1]*reciprocal.y+uv[2]*reciprocal.z)/(reciprocal.x+reciprocal.y+reciprocal.z);
    float depth=dot(weights,float3(clip[0].z/clip[0].w,clip[1].z/clip[1].w,clip[2].z/clip[2].w));
    if (abs(depthImage.Load(int3(uint2(pixel.xy),0))-depth)>2e-5) return false;
    return true;
}

[shader("fragment")]
float4 worldSceneResolve(float4 pixel : SV_Position) : SV_Target0 {
    uint2 identity=visibilityImage.Load(int3(uint2(pixel.xy),0));
    if (identity.x==0) return float4(0.025,0.07,0.12,1);
    uint chunk=identity.x>>29;
    identity.x &= 0x1fffffffu;
    if (chunk>=4 || identity.x==0) return float4(1,0,1,1);
    float2 uv;
    if (!reconstructWorldUV(pixel,identity,chunk,uv)) return float4(1,0,1,1);
    uint meshletID=identity.x-1;
    uint count=worldFactors[NonUniformResourceIndex(chunk)].Load(16), meshlets=worldFactors[NonUniformResourceIndex(chunk)].Load(20);
    if (meshletID>=meshlets) return float4(1,0,1,1);
    uint factors=96+((worldFactors[NonUniformResourceIndex(chunk)].Load(12)+3)&~3u);
    uint offsets=factors+count*16;
    uint begin=worldFactors[NonUniformResourceIndex(chunk)].Load(offsets+meshletID*4);
    uint end=worldFactors[NonUniformResourceIndex(chunk)].Load(offsets+(meshletID+1)*4);
    if (identity.y>=end-begin) return float4(1,0,1,1);
    uint material=worldFactors[NonUniformResourceIndex(chunk)].Load(offsets+(meshlets+1)*4+(begin+identity.y)*4);
    if (material>=count) return float4(1,0,1,1);
    float4 factor=asfloat(worldFactors[NonUniformResourceIndex(chunk)].Load4(factors+material*16));
    if (material>=worldTables[NonUniformResourceIndex(chunk)].Load(16)) return float4(1,0,1,1);
    uint textureCount=worldTables[NonUniformResourceIndex(chunk)].Load(20);
    uint indices=88+((worldTables[NonUniformResourceIndex(chunk)].Load(12)+3)&~3u)+textureCount*40;
    uint texture=worldTables[NonUniformResourceIndex(chunk)].Load(indices+material*4);
    if (texture>=textureCount) return float4(1,0,1,1);
    // Divergent indexing directly into the push-constant uint4 failed the
    // varied-palette replay on Slang 2026.19/RADV. Scalar component selection
    // passes the same oracle; descriptor indices remain explicitly nonuniform.
    uint base = chunk==0 ? sceneCamera.textureOffsets.x : chunk==1 ? sceneCamera.textureOffsets.y : chunk==2 ? sceneCamera.textureOffsets.z : sceneCamera.textureOffsets.w;
    return residentTextures[NonUniformResourceIndex(base+texture)].SampleLevel(residentSampler,uv,0)*factor;
}
