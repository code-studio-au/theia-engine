#include "asset_resolve.slang"
[[vk::binding(6,0)]] ByteAddressBuffer authoredUVs;
bool reconstructAuthoredUV(float4 pixel, out uint2 identity, out float2 result) {
    result=0;
    identity=visibilityImage.Load(int3(uint2(pixel.xy),0));
    if (identity.x==0) return false;
    uint meshletID=identity.x-1;
    if (meshletID>=resolveGeometry.Load(20)) return false;
    uint positionOffset=76+((resolveGeometry.Load(12)+3)&~3u);
    uint meshletOffset=positionOffset+resolveGeometry.Load(16)*12;
    uint refsOffset=meshletOffset+resolveGeometry.Load(20)*16;
    uint trianglesOffset=refsOffset+resolveGeometry.Load(24)*4;
    uint4 meshlet=resolveGeometry.Load4(meshletOffset+meshletID*16);
    if (identity.y>=meshlet.w) return false;
    uint uvOffset=88+((authoredUVs.Load(12)+3)&~3u);
    float2 uv[3]; float4 clip[3]; float2 screen[3];
    for (uint axis=0; axis<3; ++axis) {
        uint local=resolveByte(trianglesOffset+meshlet.y+identity.y*3+axis);
        uint global=resolveGeometry.Load(refsOffset+(meshlet.x+local)*4);
        if (global>=authoredUVs.Load(16)) return false;
        uv[axis]=asfloat(authoredUVs.Load2(uvOffset+global*8));
        float4 position=float4(asfloat(resolveGeometry.Load3(positionOffset+global*12)),1);
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
float4 assetUvResolveFragment(float4 pixel : SV_Position) : SV_Target0 {
    uint2 identity; float2 uv;
    if (!reconstructAuthoredUV(pixel,identity,uv))
        return identity.x==0 ? float4(0.025,0.07,0.12,1) : float4(1,0,1,1);
    return float4(saturate(uv*0.4+0.1),0.5,1);
}
