#include "triangle.slang"

// One task group publishes transforms for two downstream mesh groups.
struct TaskPayload {
    float2 offsets[2];
    float scale;
};

groupshared TaskPayload taskPayload;

[shader("amplification")]
[numthreads(32, 1, 1)]
void taskMain(uint lane : SV_GroupIndex) {
    if (lane == 0) {
        taskPayload.offsets[0] = float2(-0.5, 0.0);
        taskPayload.offsets[1] = float2(0.5, 0.0);
        taskPayload.scale = 0.45;
    }
    GroupMemoryBarrierWithGroupSync();
    // Uniform group operation, outside the lane-zero branch.
    DispatchMesh(2, 1, 1, taskPayload);
}

[shader("mesh")]
[numthreads(32, 1, 1)]
[outputtopology("triangle")]
void amplifiedMeshMain(uint lane : SV_GroupIndex, uint3 group : SV_GroupID,
                       in payload TaskPayload payload,
                       out vertices Vertex vertices[3],
                       out indices uint3 triangles[1]) {
    SetMeshOutputCounts(3, 1);
    if (lane == 0) {
        float2 offset = payload.offsets[group.x];
        vertices[0].position = float4(float2(-0.7, 0.6) * payload.scale + offset, 0.0, 1.0);
        vertices[0].color = float3(1.0, 0.0, 0.0);
        vertices[1].position = float4(float2(0.7, 0.6) * payload.scale + offset, 0.0, 1.0);
        vertices[1].color = float3(0.0, 1.0, 0.0);
        vertices[2].position = float4(float2(0.0, -0.6) * payload.scale + offset, 0.0, 1.0);
        vertices[2].color = float3(0.0, 0.0, 1.0);
        triangles[0] = uint3(0, 1, 2);
    }
}
