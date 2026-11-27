#include "world_ground_query.slang"
[[vk::binding(1,0)]] ByteAddressBuffer groundQueries;
[[vk::binding(2,0)]] RWByteAddressBuffer groundResults;
[shader("compute")]
[numthreads(64,1,1)]
void worldGroundCompute(uint3 thread:SV_DispatchThreadID) {
    uint size;groundQueries.GetDimensions(size);uint id=thread.x;if(id>=size/16)return;
    uint4 query=groundQueries.Load4(id*16);
    groundResults.Store4(id*16,queryGround(asfloat(query.xy),query.z));
}
