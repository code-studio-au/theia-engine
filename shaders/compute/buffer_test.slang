[[vk::binding(0, 0)]]
RWStructuredBuffer<uint> outputBuffer;

[shader("compute")]
[numthreads(64, 1, 1)]
void computeMain(uint3 threadId : SV_DispatchThreadID)
{
    uint index = threadId.x;

    if (index < 256)
    {
        outputBuffer[index] = index * 2 + 1;
    }
}
