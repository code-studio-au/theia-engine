#include "resolve.slang"
[[vk::binding(4,0)]] ByteAddressBuffer resolveGeometry;
struct SceneCameraRows {
    float4 row0; float4 row1; float4 row2; float4 row3;
#ifdef THEIA_COOKED_HUD
    uint4 hudPerf; uint4 hudScene; uint4 hudCounts;
#endif
#ifdef THEIA_TEXTURE_MIP_TAIL
    uint4 textureTail; // original width/height, first resident source level, source count
#endif
};
[[vk::push_constant]] ConstantBuffer<SceneCameraRows> sceneCamera;
uint resolveByte(uint offset) { return (resolveGeometry.Load(offset&~3u)>>((offset&3u)*8))&255u; }
[shader("fragment")]
float4 assetResolveFragment(float4 pixel : SV_Position) : SV_Target0 {
    uint2 identity=visibilityImage.Load(int3(uint2(pixel.xy),0));
    if (identity.x==0) return float4(0.025,0.07,0.12,1);
    uint meshletID=identity.x-1;
    if (meshletID>=resolveGeometry.Load(20)) return float4(1,0,1,1);
    uint positionOffset=76+((resolveGeometry.Load(12)+3)&~3u);
    uint meshletOffset=positionOffset+resolveGeometry.Load(16)*12;
    uint refsOffset=meshletOffset+resolveGeometry.Load(20)*16;
    uint trianglesOffset=refsOffset+resolveGeometry.Load(24)*4;
    uint4 meshlet=resolveGeometry.Load4(meshletOffset+meshletID*16);
    if (identity.y>=meshlet.w) return float4(1,0,1,1);
    float3 world[3]; float4 clip[3]; float2 screen[3];
    for (uint axis=0; axis<3; ++axis) {
        uint local=resolveByte(trianglesOffset+meshlet.y+identity.y*3+axis);
        uint global=resolveGeometry.Load(refsOffset+(meshlet.x+local)*4);
        world[axis]=asfloat(resolveGeometry.Load3(positionOffset+global*12));
        float4 position=float4(world[axis],1);
        clip[axis]=float4(dot(sceneCamera.row0,position),dot(sceneCamera.row1,position),dot(sceneCamera.row2,position),dot(sceneCamera.row3,position));
        screen[axis]=clip[axis].xy/clip[axis].w;
    }
    uint width,height; visibilityImage.GetDimensions(width,height);
    float2 point=pixel.xy/float2(width,height)*2-1;
    float2 a=screen[0],b=screen[1],c=screen[2];
    float denominator=(b.y-c.y)*(a.x-c.x)+(c.x-b.x)*(a.y-c.y);
    if (abs(denominator)<1e-10) return float4(1,0,1,1);
    float u=((b.y-c.y)*(point.x-c.x)+(c.x-b.x)*(point.y-c.y))/denominator;
    float v=((c.y-a.y)*(point.x-c.x)+(a.x-c.x)*(point.y-c.y))/denominator;
    float3 weights=float3(u,v,1-u-v);
    float3 reciprocal=weights/float3(clip[0].w,clip[1].w,clip[2].w);
    float3 position=(world[0]*reciprocal.x+world[1]*reciprocal.y+world[2]*reciprocal.z)/(reciprocal.x+reciprocal.y+reciprocal.z);
    float expectedDepth=dot(weights,float3(clip[0].z/clip[0].w,clip[1].z/clip[1].w,clip[2].z/clip[2].w));
    if (abs(depthImage.Load(int3(uint2(pixel.xy),0))-expectedDepth)>2e-5) return float4(1,0,1,1);
    float3 material=materialImages[NonUniformResourceIndex(meshletID&1)].SampleLevel(materialSampler,float2(v,1-u-v),0).rgb;
    return float4(float3(position.xy*0.5+0.5,0.5)*material,1);
}
