#include "asset_uv_resolve.slang"
[[vk::binding(5,0)]] ByteAddressBuffer authoredFactors;
[[vk::binding(7,0)]] Texture2D<float4> residentTextures[];
[[vk::binding(8,0)]] SamplerState residentSampler;
[[vk::binding(9,0)]] ByteAddressBuffer materialTextures;
[shader("fragment")]
float4 assetTextureTableResolveFragment(float4 pixel : SV_Position) : SV_Target0 {
    uint2 identity; float2 uv;
    if (!reconstructAuthoredUV(pixel,identity,uv))
        return identity.x==0 ? float4(0.025,0.07,0.12,1) : float4(1,0,1,1);
    uint meshletID=identity.x-1;
    uint count=authoredFactors.Load(16), meshlets=authoredFactors.Load(20);
    if (meshletID>=meshlets) return float4(1,0,1,1);
    uint factors=96+((authoredFactors.Load(12)+3)&~3u);
    uint offsets=factors+count*16;
    uint begin=authoredFactors.Load(offsets+meshletID*4);
    uint end=authoredFactors.Load(offsets+(meshletID+1)*4);
    if (identity.y>=end-begin) return float4(1,0,1,1);
    uint material=authoredFactors.Load(offsets+(meshlets+1)*4+(begin+identity.y)*4);
    if (material>=count) return float4(1,0,1,1);
    float4 factor=asfloat(authoredFactors.Load4(factors+material*16));
    if (material>=materialTextures.Load(16)) return float4(1,0,1,1);
    uint textureCount=materialTextures.Load(20);
    uint indices=88+((materialTextures.Load(12)+3)&~3u)+textureCount*40;
    uint texture=materialTextures.Load(indices+material*4);
    if (texture>=textureCount) return float4(1,0,1,1);
    return residentTextures[NonUniformResourceIndex(texture)].SampleLevel(residentSampler,uv,0)*factor;
}
