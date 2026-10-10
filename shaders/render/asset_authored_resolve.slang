#include "resolve.slang"
[[vk::binding(5,0)]] ByteAddressBuffer authoredMaterials;
[shader("fragment")]
float4 assetAuthoredResolveFragment(float4 pixel : SV_Position) : SV_Target0 {
    uint2 identity=visibilityImage.Load(int3(uint2(pixel.xy),0));
    if (identity.x==0) return float4(0.025,0.07,0.12,1);
    uint meshletID=identity.x-1;
    uint materialCount=authoredMaterials.Load(16);
    uint meshletCount=authoredMaterials.Load(20);
    if (meshletID>=meshletCount) return float4(1,0,1,1);
    uint materialsOffset=96+((authoredMaterials.Load(12)+3)&~3u);
    uint offsetsOffset=materialsOffset+materialCount*16;
    uint begin=authoredMaterials.Load(offsetsOffset+meshletID*4);
    uint end=authoredMaterials.Load(offsetsOffset+(meshletID+1)*4);
    if (identity.y>=end-begin) return float4(1,0,1,1);
    uint assignmentsOffset=offsetsOffset+(meshletCount+1)*4;
    uint material=authoredMaterials.Load(assignmentsOffset+(begin+identity.y)*4);
    if (material>=materialCount) return float4(1,0,1,1);
    return asfloat(authoredMaterials.Load4(materialsOffset+material*16));
}
