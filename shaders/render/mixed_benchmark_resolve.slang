// Experimental frame-local identity namespace: bit31 denotes visual proxies.
#define THEIA_MIXED_WORLD 1
#include "asset_texture_table_resolve.slang"
[[vk::binding(11,0)]] ByteAddressBuffer mixedProxyWork;
#ifdef THEIA_COOKED_HUD
[[vk::binding(10,0)]] ByteAddressBuffer cookedHudWork;
#include "benchmark_hud.slang"
#endif
[shader("fragment")]
float4 mixedBenchmarkResolve(float4 pixel:SV_Position):SV_Target0 {
#ifdef THEIA_COOKED_HUD
    if(sceneCamera.hudPerf.w!=0 && pixel.x>=12 && pixel.x<420 && pixel.y>=12 && pixel.y<300)
        return hudPixel(uint2(pixel.xy));
#endif
    uint2 identity=visibilityImage.Load(int3(uint2(pixel.xy),0));
    if((identity.x&0x80000000u)==0) return assetTextureTableResolveFragment(pixel);
    uint id=(identity.x&0x7fffffffu)-1,primitive=identity.y;
    float fraction=float(id%5)/4;
    float3 color=lerp(float3(.15,.3,.8),float3(.8,.15,.05),fraction);
    float light[6]={.35,1,.55,.8,.5,.75};
    float lighting=primitive<12 ? light[primitive/2]:.7;
    float depth=depthImage.Load(int3(uint2(pixel.xy),0));
    float distance=25./max(.1,250.-depth*249.9);
    return float4(lerp(color*lighting,float3(.12,.23,.35),clamp(distance/180.,0,.7)),1);
}
