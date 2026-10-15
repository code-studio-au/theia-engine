[[vk::binding(0,0)]] Texture2D<uint2> benchmarkIdentity;
[[vk::binding(1,0)]] Texture2D<float> benchmarkDepth;
[[vk::binding(4,0)]] ByteAddressBuffer benchmarkInstances;
#include "benchmark_hud.slang"
[shader("fragment")]
float4 benchmarkResolve(float4 pixel:SV_Position):SV_Target0 {
    if(hud.perf.w!=0 && pixel.x>=12 && pixel.x<420 && pixel.y>=12 && pixel.y<252) return hudPixel(uint2(pixel.xy));
    uint2 identity=benchmarkIdentity.Load(int3(uint2(pixel.xy),0));
    if(identity.x==0) return float4(0.12,0.23,0.35,1);
    uint id=identity.x-1,kind=benchmarkInstances.Load(41824+id*32+12);
    float3 colors[4]={float3(0.12,0.3,0.08),float3(0.55,0.38,0.22),float3(0.06,0.23,0.06),float3(0.15,0.3,0.8)};
    float3 color=colors[min(kind,3u)];
    if(kind==3) color=lerp(color,float3(0.8,0.15,0.05),float(id%5)/4);
    float lighting[6]={0.35,1,0.55,0.8,0.5,0.75};
    float light=kind==0 ? 0.85:(identity.y<12 ? lighting[identity.y/2]:0.7);
    if(identity.y>=12 && kind==1) color=float3(0.3,0.12,0.06);
    float depth=benchmarkDepth.Load(int3(uint2(pixel.xy),0));
    float distance=25./max(0.1,250.-depth*249.9);
    float fog=clamp(distance/180.,0,0.7);
    return float4(lerp(color*light,float3(0.12,0.23,0.35),fog),1);
}
