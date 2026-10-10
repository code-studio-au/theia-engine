#include "amplified.slang"

// Explicit byte ABI: command uint3 at 0, draw count at 12, offsets at 16/24,
// scale at 32, downstream mesh count at 36. Host never maps this buffer.
[[vk::binding(0, 0)]] RWByteAddressBuffer work;
struct Parameters { uint requestedItems; };
[[vk::push_constant]] ConstantBuffer<Parameters> parameters;

[shader("compute")]
[numthreads(1, 1, 1)]
void generateWork() {
    uint items = min(parameters.requestedItems, 2u);
    work.Store3(0, uint3(items > 0 ? 1 : 0, 1, 1));
    work.Store(12, items > 0 ? 1 : 0);
    work.Store2(16, asuint(float2(-0.5, 0.0)));
    work.Store2(24, asuint(float2(0.5, 0.0)));
    work.Store(32, asuint(0.45));
    work.Store(36, items);
}

groupshared uint meshCount;
[shader("amplification")]
[numthreads(32, 1, 1)]
void indirectTaskMain(uint lane : SV_GroupIndex) {
    if (lane == 0) {
        taskPayload.offsets[0] = asfloat(work.Load2(16));
        taskPayload.offsets[1] = asfloat(work.Load2(24));
        taskPayload.scale = asfloat(work.Load(32));
        meshCount = min(work.Load(36), 2u);
    }
    GroupMemoryBarrierWithGroupSync();
    DispatchMesh(meshCount, 1, 1, taskPayload);
}
