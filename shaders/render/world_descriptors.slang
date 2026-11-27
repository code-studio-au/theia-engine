// Optional nonuniform storage descriptor-array qualification; no gameplay state.
[[vk::binding(0,0)]] ByteAddressBuffer sourceBuffers[];
[[vk::binding(1,0)]] RWByteAddressBuffer resultBuffer;
[numthreads(64,1,1)]
void worldDescriptorCompute(uint3 thread : SV_DispatchThreadID) {
    uint descriptor=NonUniformResourceIndex(thread.x%4);
    uint word=(thread.x/4)%4;
    resultBuffer.Store(thread.x*4,sourceBuffers[descriptor].Load(word*4));
}
