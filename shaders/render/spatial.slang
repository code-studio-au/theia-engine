#include "triangle.slang"
// One 64-lane qualification group builds 8x8 direct-addressed XY CSR memberships.
// 32 GPU-initialized resident bounds; complete XY bounds remain inside [-1,1].
[[vk::binding(0,0)]] RWByteAddressBuffer spatialWork;
struct SpatialInput { uint scenario; };
[[vk::push_constant]] ConstantBuffer<SpatialInput> spatialInput;
static const uint IDS=32, BOUNDS=160, COUNTS=928, OFFSETS=1184, MEMBERS=1444;
groupshared uint counts[64];
groupshared uint offsets[65];
groupshared uint candidateMask;
groupshared uint visibleCount;
bool overlap3(float3 a, float3 b, float3 c, float3 d) { return all(a <= d) && all(b >= c); }
float3 minimum(uint id) { return asfloat(spatialWork.Load3(BOUNDS + id * 24)); }
float3 maximum(uint id) { return asfloat(spatialWork.Load3(BOUNDS + id * 24 + 12)); }
[numthreads(64,1,1)]
void spatialCompute(uint lane : SV_GroupIndex) {
    if (lane < 32) {
        float3 center = float3(-0.875 + (lane % 8) * 0.25, -0.75 + (lane / 8) * 0.5, 0.125 + (lane % 4) * 0.25);
        spatialWork.Store3(BOUNDS + lane * 24, asuint(center - float3(0.06,0.06,0.04)));
        spatialWork.Store3(BOUNDS + lane * 24 + 12, asuint(center + float3(0.06,0.06,0.04)));
    }
    if (lane == 0) { candidateMask = 0; visibleCount = 0; }
    AllMemoryBarrierWithGroupSync();
    float2 cellMin = float2(-1.0 + (lane % 8) * 0.25, -1.0 + (lane / 8) * 0.25);
    float2 cellMax = cellMin + 0.25;
    uint count = 0;
    for (uint id = 0; id < 32; ++id)
        if (all(minimum(id).xy <= cellMax) && all(maximum(id).xy >= cellMin)) ++count;
    counts[lane] = count;
    spatialWork.Store(COUNTS + lane * 4, count);
    GroupMemoryBarrierWithGroupSync();
    if (lane == 0) {
        uint total = 0;
        for (uint cell = 0; cell < 64; ++cell) { offsets[cell] = total; total += counts[cell]; }
        offsets[64] = total;
        spatialWork.Store(28, total);
        // Capacity is conservative 64*32, independent of occupancy of this fixture.
        spatialWork.Store(24, total > 2048 ? 1 : 0);
    }
    GroupMemoryBarrierWithGroupSync();
    spatialWork.Store(OFFSETS + lane * 4, offsets[lane]);
    if (lane == 0) spatialWork.Store(OFFSETS + 256, offsets[64]);
    uint cursor = offsets[lane];
    for (uint id = 0; id < 32; ++id)
        if (all(minimum(id).xy <= cellMax) && all(maximum(id).xy >= cellMin)) spatialWork.Store(MEMBERS + (cursor++) * 4, id);
    float3 viewMin = spatialInput.scenario == 0 ? float3(2,2,2) : spatialInput.scenario == 1 ? float3(-1,-1,-0.05) : float3(-1,-1,-1);
    float3 viewMax = spatialInput.scenario == 0 ? float3(3,3,3) : spatialInput.scenario == 1 ? float3(0,1,0.5) : float3(1,1,1);
    AllMemoryBarrierWithGroupSync();
    if (all(viewMin.xy <= cellMax) && all(viewMax.xy >= cellMin)) {
        for (uint m = offsets[lane]; m < offsets[lane+1]; ++m) {
            uint id = spatialWork.Load(MEMBERS + m * 4);
            InterlockedOr(candidateMask, 1u << id);
        }
    }
    GroupMemoryBarrierWithGroupSync();
    if (lane < 32 && (candidateMask & (1u << lane)) != 0 && overlap3(minimum(lane),maximum(lane),viewMin,viewMax)) {
        uint output;
        InterlockedAdd(visibleCount, 1, output);
        spatialWork.Store(IDS + output * 4, lane);
    }
    AllMemoryBarrierWithGroupSync();
    if (lane == 0) {
        uint drawCount = visibleCount != 0 ? 1 : 0;
        spatialWork.Store3(0, uint3(drawCount,1,1));
        spatialWork.Store(12,drawCount); spatialWork.Store(16,visibleCount); spatialWork.Store(20,candidateMask);
    }
}
struct SpatialPayload { uint count; };
groupshared SpatialPayload spatialPayload;
[shader("amplification")]
[numthreads(32,1,1)]
void spatialTask(uint lane : SV_GroupIndex) {
    if (lane == 0) spatialPayload.count = spatialWork.Load(16);
    GroupMemoryBarrierWithGroupSync();
    DispatchMesh(spatialPayload.count,1,1,spatialPayload);
}
[shader("mesh")]
[numthreads(32,1,1)]
[outputtopology("triangle")]
void spatialMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID,
    in payload SpatialPayload payload, out vertices Vertex vertices[3], out indices uint3 triangles[1]) {
    SetMeshOutputCounts(group.x < payload.count ? 3 : 0, group.x < payload.count ? 1 : 0);
    if (lane == 0 && group.x < payload.count) {
        uint id = spatialWork.Load(IDS + group.x * 4);
        float2 center = (minimum(id).xy + maximum(id).xy) * 0.5;
        vertices[0].position = float4(center + float2(-0.06,0.06),0,1); vertices[0].color = float3(1,0,0);
        vertices[1].position = float4(center + float2(0.06,0.06),0,1); vertices[1].color = float3(0,1,0);
        vertices[2].position = float4(center + float2(0,-0.06),0,1); vertices[2].color = float3(0,0,1);
        triangles[0] = uint3(0,1,2);
    }
}
