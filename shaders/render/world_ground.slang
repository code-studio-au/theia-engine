// Bounded presentation heightfield query. No gameplay collision/authority.
[[vk::binding(0,0)]] ByteAddressBuffer groundTable;
[[vk::binding(1,0)]] ByteAddressBuffer groundQueries;
[[vk::binding(2,0)]] RWByteAddressBuffer groundResults;
float cross2(float2 a,float2 b) {return a.x*b.y-a.y*b.x;}
[shader("compute")]
[numthreads(64,1,1)]
void worldGroundCompute(uint3 thread:SV_DispatchThreadID) {
    uint size;groundQueries.GetDimensions(size);uint id=thread.x;if(id>=size/16)return;
    uint4 query=groundQueries.Load4(id*16);float2 p=asfloat(query.xy);uint mask=query.z;
    if(!all(isfinite(p)) || any(abs(p)>64) || mask>15) {
        groundResults.Store4(id*16,uint4(0,0,4,0));return;
    }
    uint region=(p.x>=0 ? 1u:0u)+(p.y>=0 ? 2u:0u);
    uint surface=region+((mask&(1u<<region))!=0 ? 0u:4u);
    uint header=surface*32;float2 origin=asfloat(groundTable.Load2(header));
    uint triangles=groundTable.Load(header+8),offsets=groundTable.Load(header+12),references=groundTable.Load(header+16);
    uint2 cell=min(uint2(floor((p-origin)/8)),uint2(7,7));uint index=cell.y*8+cell.x;
    uint start=groundTable.Load(offsets+index*4),end=groundTable.Load(offsets+(index+1)*4);
    float result=0;uint status=1;
    for(uint i=start;i<end;++i) {
        uint address=triangles+groundTable.Load(references+i*4)*48;
        float3 a=asfloat(groundTable.Load3(address)),b=asfloat(groundTable.Load3(address+16)),c=asfloat(groundTable.Load3(address+32));
        float2 ab=b.xy-a.xy,ac=c.xy-a.xy,ap=p-a.xy;
        float denominator=cross2(ab,ac),u=cross2(ap,ac)/denominator,v=cross2(ab,ap)/denominator;
        if(u>=-0.000001 && v>=-0.000001 && u+v<=1.000001) {
            float value=a.z+u*(b.z-a.z)+v*(c.z-a.z);
            if(status==0 && abs(result-value)>0.00001)status=2;
            if(status!=2) {result=value;status=0;}
        }
    }
    groundResults.Store4(id*16,uint4(asuint(result),region,status,end-start));
}
