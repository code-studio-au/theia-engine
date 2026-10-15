#include "asset_texture_table_resolve.slang"
[[vk::binding(12,0)]] ByteAddressBuffer secondGeometry;
[[vk::binding(13,0)]] ByteAddressBuffer secondFactors;
[[vk::binding(14,0)]] ByteAddressBuffer secondUVs;
[[vk::binding(15,0)]] ByteAddressBuffer secondMaterialTextures;
[[vk::binding(16,0)]] Texture2D<float4> secondTextures[];
[[vk::binding(17,0)]] SamplerState secondSampler;
uint secondByte(uint offset) { return (secondGeometry.Load(offset&~3u)>>((offset&3u)*8))&255u; }
bool reconstructSecondUV(float4 pixel, out uint2 identity, out float2 result) {
    result=0;
    identity=visibilityImage.Load(int3(uint2(pixel.xy),0));
    identity.x &= 0x3fffffffu;
    if (identity.x==0) return false;
    uint meshletID=identity.x-1;
    if (meshletID>=secondGeometry.Load(20)) return false;
    uint positionOffset=76+((secondGeometry.Load(12)+3)&~3u);
    uint meshletOffset=positionOffset+secondGeometry.Load(16)*12;
    uint refsOffset=meshletOffset+secondGeometry.Load(20)*16;
    uint trianglesOffset=refsOffset+secondGeometry.Load(24)*4;
    uint4 meshlet=secondGeometry.Load4(meshletOffset+meshletID*16);
    if (identity.y>=meshlet.w) return false;
    uint uvOffset=88+((secondUVs.Load(12)+3)&~3u);
    float2 uv[3]; float4 clip[3]; float2 screen[3];
    for (uint axis=0; axis<3; ++axis) {
        uint local=secondByte(trianglesOffset+meshlet.y+identity.y*3+axis);
        uint global=secondGeometry.Load(refsOffset+(meshlet.x+local)*4);
        if (global>=secondUVs.Load(16)) return false;
        uv[axis]=asfloat(secondUVs.Load2(uvOffset+global*8));
        float4 position=float4(asfloat(secondGeometry.Load3(positionOffset+global*12)),1);
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
float4 secondTextureResolve(float4 pixel : SV_Position) : SV_Target0 {
    uint2 identity; float2 uv;
    if (!reconstructSecondUV(pixel,identity,uv))
        return identity.x==0 ? float4(0.025,0.07,0.12,1) : float4(1,0,1,1);
    uint meshletID=identity.x-1;
    uint count=secondFactors.Load(16), meshlets=secondFactors.Load(20);
    if (meshletID>=meshlets) return float4(1,0,1,1);
    uint factors=96+((secondFactors.Load(12)+3)&~3u);
    uint offsets=factors+count*16;
    uint begin=secondFactors.Load(offsets+meshletID*4);
    uint end=secondFactors.Load(offsets+(meshletID+1)*4);
    if (identity.y>=end-begin) return float4(1,0,1,1);
    uint material=secondFactors.Load(offsets+(meshlets+1)*4+(begin+identity.y)*4);
    if (material>=count) return float4(1,0,1,1);
    float4 factor=asfloat(secondFactors.Load4(factors+material*16));
    if (material>=secondMaterialTextures.Load(16)) return float4(1,0,1,1);
    uint textureCount=secondMaterialTextures.Load(20);
    uint indices=88+((secondMaterialTextures.Load(12)+3)&~3u)+textureCount*40;
    uint texture=secondMaterialTextures.Load(indices+material*4);
    if (texture>=textureCount) return float4(1,0,1,1);
    return secondTextures[NonUniformResourceIndex(texture)].SampleLevel(secondSampler,uv,0)*factor;
}

[shader("fragment")]
float4 worldPairTexturedResolve(float4 pixel : SV_Position) : SV_Target0 {
    uint identity=visibilityImage.Load(int3(uint2(pixel.xy),0)).x;
    if ((identity>>30)>1) return float4(1,0,1,1);
    if ((identity&0x40000000u)!=0) return secondTextureResolve(pixel);
    return assetTextureTableResolveFragment(pixel);
}
