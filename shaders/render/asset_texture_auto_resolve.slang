#include "asset_uv_resolve.slang"
[[vk::binding(5,0)]] ByteAddressBuffer autoFactors;
[[vk::binding(7,0)]] Texture2D<float4> autoTexture;
[[vk::binding(8,0)]] SamplerState autoSampler;
[[vk::binding(9,0)]] RWByteAddressBuffer footprintProbe;

// Diagnostic only: one record at each of the 9x9 qualification pixel locations.
// Every record is written once per full-screen resolve, including background.
void writeFootprint(float4 pixel,uint2 identity,float2 uv,float2 dx,float2 dy,float rho,uint mip,uint reason) {
    uint width,height;visibilityImage.GetDimensions(width,height);
    if(width<10 || height<10 || width>16384 || height>16384) return;
    uint2 p=uint2(pixel.xy);
    uint2 g=(p*10+uint2(width,height)-1)/uint2(width,height);
    if(any(g<1) || any(g>9) || any(p!=uint2(width,height)*g/10)) return;
    uint offset=((g.y-1)*9+g.x-1)*48;
    footprintProbe.Store4(offset,asuint(float4(dx,dy)));
    footprintProbe.Store2(offset+16,asuint(uv));footprintProbe.Store(offset+24,asuint(rho));
    footprintProbe.Store(offset+28,mip);footprintProbe.Store2(offset+32,identity);
    footprintProbe.Store(offset+40,reason);
#ifdef THEIA_TEXTURE_MIP_TAIL
    footprintProbe.Store(offset+44,(reason==1 || reason==3) ? 0u:mip-sceneCamera.textureTail.z);
#else
    footprintProbe.Store(offset+44,0);
#endif
}

bool authoredGradient(float4 pixel,uint2 identity,float2 value,out float2 dx,out float2 dy) {
    dx=0;dy=0;
    uint meshletID=identity.x-1;
    uint positions=76+((resolveGeometry.Load(12)+3)&~3u);
    uint meshlets=positions+resolveGeometry.Load(16)*12;
    uint refs=meshlets+resolveGeometry.Load(20)*16;
    uint triangles=refs+resolveGeometry.Load(24)*4;
    uint4 meshlet=resolveGeometry.Load4(meshlets+meshletID*16);
    uint uvOffset=88+((authoredUVs.Load(12)+3)&~3u);
    float2 coords[3];float4 clip[3];float2 screen[3];
    for(uint i=0;i<3;++i) {
        uint vertex=resolveGeometry.Load(refs+(meshlet.x+resolveByte(triangles+meshlet.y+identity.y*3+i))*4);
        coords[i]=asfloat(authoredUVs.Load2(uvOffset+vertex*8));
        float4 p=float4(asfloat(resolveGeometry.Load3(positions+vertex*12)),1);
        clip[i]=float4(dot(sceneCamera.row0,p),dot(sceneCamera.row1,p),dot(sceneCamera.row2,p),dot(sceneCamera.row3,p));
        if(!all(abs(clip[i])<=64) || !all(abs(coords[i])<=64) || clip[i].w<1e-4 || clip[i].z<0 || clip[i].z>clip[i].w) return false;
        screen[i]=clip[i].xy/clip[i].w;
    }
    uint width,height;visibilityImage.GetDimensions(width,height);
    if(width>16384 || height>16384) return false;
    float2 a=screen[0],b=screen[1],c=screen[2];
    float area=(b.y-c.y)*(a.x-c.x)+(c.x-b.x)*(a.y-c.y);
    if(!isfinite(area) || abs(area)<1e-10) return false;
    float2 point=pixel.xy/float2(width,height)*2-1;
    float u=((b.y-c.y)*(point.x-c.x)+(c.x-b.x)*(point.y-c.y))/area;
    float v=((c.y-a.y)*(point.x-c.x)+(a.x-c.x)*(point.y-c.y))/area;
    float3 inverse=1/float3(clip[0].w,clip[1].w,clip[2].w);
    float r=dot(float3(u,v,1-u-v),inverse);
    if(!isfinite(r) || r<=1e-10) return false;
    float2 x=float2(b.y-c.y,c.y-a.y)/area*2/float(width);
    float2 y=float2(c.x-b.x,a.x-c.x)/area*2/float(height);
    float3 gx=float3(x,-x.x-x.y)*inverse;
    float3 gy=float3(y,-y.x-y.y)*inverse;
    dx=(coords[0]*gx.x+coords[1]*gx.y+coords[2]*gx.z-value*(gx.x+gx.y+gx.z))/r;
    dy=(coords[0]*gy.x+coords[1]*gy.y+coords[2]*gy.z-value*(gy.x+gy.y+gy.z))/r;
    return all(isfinite(dx)) && all(isfinite(dy));
}

[shader("fragment")]
float4 assetTextureAutoResolveFragment(float4 pixel:SV_Position):SV_Target0 {
    uint2 identity;float2 uv;
    if(!reconstructAuthoredUV(pixel,identity,uv)) {
        writeFootprint(pixel,identity,0,0,0,0,0,identity.x==0 ? 1u:3u);
        return identity.x==0 ? float4(0.025,0.07,0.12,1):float4(1,0,1,1);
    }
    if(!all(isfinite(uv))) {writeFootprint(pixel,identity,0,0,0,0,0,3);return float4(1,0,1,1);}
    float2 dx,dy;bool supported=authoredGradient(pixel,identity,uv,dx,dy);
    uint width,height,levels;autoTexture.GetDimensions(0,width,height,levels);
    uint first=0;
#ifdef THEIA_TEXTURE_MIP_TAIL
    uint4 tail=sceneCamera.textureTail;
    if(tail.x==0 || tail.y==0 || tail.x>1024 || tail.y>1024 || tail.w==0 || tail.w>11 || tail.z>=tail.w
        || levels!=tail.w-tail.z || width!=max(1u,tail.x>>tail.z) || height!=max(1u,tail.y>>tail.z)) {
        writeFootprint(pixel,identity,uv,0,0,0,0,3);return float4(1,0,1,1);
    }
    width=tail.x;height=tail.y;levels=tail.w;first=tail.z;
#endif
    float rho=supported ? max(length(dx*float2(width,height)),length(dy*float2(width,height))):0;
    if(!isfinite(rho)) {supported=false;rho=0;}
    if(!supported) {dx=0;dy=0;}
    uint mip=0;
    if(supported) while(mip+1<levels && rho>float(1u<<mip)) ++mip;
    bool coarsened=mip<first;mip=max(mip,first);
    uint count=autoFactors.Load(16),meshletCount=autoFactors.Load(20);
    if(identity.x>meshletCount) {
        writeFootprint(pixel,identity,uv,dx,dy,rho,mip,3);return float4(1,0,1,1);
    }
    uint factors=96+((autoFactors.Load(12)+3)&~3u),offsets=factors+count*16;
    uint begin=autoFactors.Load(offsets+(identity.x-1)*4);
    uint end=autoFactors.Load(offsets+identity.x*4);
    if(identity.y>=end-begin) {
        writeFootprint(pixel,identity,uv,dx,dy,rho,mip,3);return float4(1,0,1,1);
    }
    uint material=autoFactors.Load(offsets+(meshletCount+1)*4+(begin+identity.y)*4);
    if(material>=count) {writeFootprint(pixel,identity,uv,dx,dy,rho,mip,3);return float4(1,0,1,1);}
    writeFootprint(pixel,identity,uv,dx,dy,rho,mip,supported ? (coarsened ? 4u:0u):2u);
    return autoTexture.SampleLevel(autoSampler,uv,float(mip-first))*asfloat(autoFactors.Load4(factors+material*16));
}
