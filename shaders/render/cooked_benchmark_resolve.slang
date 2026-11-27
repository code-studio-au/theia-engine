// Debug stats are a separate optional pipeline; normal cooked resolve stays unchanged.
#define THEIA_COOKED_HUD 1
#include "asset_texture_table_resolve.slang"
[[vk::binding(10,0)]] ByteAddressBuffer cookedHudWork;
#include "benchmark_hud.slang"
[shader("fragment")]
float4 cookedBenchmarkResolve(float4 pixel:SV_Position):SV_Target0 {
    if(sceneCamera.hudPerf.w!=0 && pixel.x>=12 && pixel.x<420 && pixel.y>=12 && pixel.y<252)
        return hudPixel(uint2(pixel.xy));
    return assetTextureTableResolveFragment(pixel);
}
