[[vk::binding(0,0)]] ByteAddressBuffer visibilityWork;
struct VisibilityPayload { uint count; };
groupshared VisibilityPayload visibilityPayload;
struct VisibilityVertex {
    float4 position : SV_Position;
    nointerpolation uint2 identity : TEXCOORD0;
};
[shader("amplification")]
[numthreads(32,1,1)]
void visibilityTask(uint lane : SV_GroupIndex) {
    if (lane == 0) visibilityPayload.count = visibilityWork.Load(36);
    GroupMemoryBarrierWithGroupSync();
    DispatchMesh(visibilityPayload.count,1,1,visibilityPayload);
}
[shader("mesh")]
[numthreads(32,1,1)]
[outputtopology("triangle")]
void visibilityMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID,
    in payload VisibilityPayload payload,
    out vertices VisibilityVertex vertices[3], out indices uint3 triangles[1]) {
    SetMeshOutputCounts(group.x < payload.count ? 3 : 0, group.x < payload.count ? 1 : 0);
    if (lane == 0 && group.x < payload.count) {
        float scale = group.x == 0 ? 0.5 : 1.0;
        float depth = group.x == 0 ? 0.25 : 0.75;
        vertices[0].position = float4(float2(-0.7,0.6)*scale,depth,1);
        vertices[1].position = float4(float2(0.7,0.6)*scale,depth,1);
        vertices[2].position = float4(float2(0,-0.6)*scale,depth,1);
        for (uint vertex = 0; vertex < 3; ++vertex) vertices[vertex].identity = uint2(group.x + 1,0);
        triangles[0] = uint3(0,1,2);
    }
}
[shader("fragment")]
uint2 visibilityFragment(VisibilityVertex input) : SV_Target0 { return input.identity; }
