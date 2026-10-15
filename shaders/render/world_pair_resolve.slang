#include "resolve.slang"
[[vk::binding(4,0)]] ByteAddressBuffer firstGeometry;
[[vk::binding(12,0)]] ByteAddressBuffer secondGeometry;
uint worldPrimitiveCount(ByteAddressBuffer geometry, uint meshlet) {
    if (meshlet>=geometry.Load(20)) return 0;
    uint offset=76+((geometry.Load(12)+3)&~3u)+geometry.Load(16)*12;
    return geometry.Load(offset+meshlet*16+12);
}
[shader("fragment")]
float4 worldPairResolve(float4 pixel : SV_Position) : SV_Target0 {
    uint2 id=visibilityImage.Load(int3(uint2(pixel.xy),0));
    if (id.x==0) return float4(0.025,0.07,0.12,1);
    uint chunk=id.x>>30;
    uint local=(id.x&0x3fffffffu);
    if (chunk>1 || local==0) return float4(1,0,1,1);
    uint count=chunk==0 ? worldPrimitiveCount(firstGeometry,local-1)
                        : worldPrimitiveCount(secondGeometry,local-1);
    if (id.y>=count) return float4(1,0,1,1);
    float3 tint=chunk==0 ? float3(0.18,0.48,0.24) : float3(0.48,0.24,0.18);
    return float4(tint*(id.y%2==0 ? 1.0 : 0.7),1);
}
