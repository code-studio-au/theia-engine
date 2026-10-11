// Bounded readonly authority presentation. One mesh group per actor/obstacle;
// interpolation is GPU presentation only, never movement or combat authority.
struct Presentation {
    float4 previousXY;
    float4 currentXY;
    uint4 logicalIds; // low/high pairs; dense position is not identity
    uint2 health;
    float alpha;
    uint blendMask;
};
[[vk::push_constant]] ConstantBuffer<Presentation> presentation;
struct Vertex {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 color : COLOR0;
};
[shader("mesh")]
[numthreads(32,1,1)]
[outputtopology("triangle")]
void authorityMesh(uint lane : SV_GroupIndex, uint3 group : SV_GroupID,
                   out vertices Vertex vertices[4], out indices uint3 triangles[2]) {
    SetMeshOutputCounts(4,2);
    if (lane == 0) {
        uint index = group.x;
        float2 center = float2(0,0);
        float halfExtent = 100.0;
        float3 color = float3(0.3,0.3,0.3);
        if (index < 2) {
            float2 previous = index == 0 ? presentation.previousXY.xy : presentation.previousXY.zw;
            float2 current = index == 0 ? presentation.currentXY.xy : presentation.currentXY.zw;
            float alpha = (presentation.blendMask & (1u << index)) != 0 ? presentation.alpha : 1.0;
            center = lerp(previous,current,alpha);
            halfExtent = 250.0;
            uint2 id = index == 0 ? presentation.logicalIds.xy : presentation.logicalIds.zw;
            uint health = presentation.health[index];
            float3 base = ((id.x ^ id.y) & 2u) != 0 ? float3(0.1,0.8,0.2) : float3(0.8,0.2,0.1);
            color = health == 0 ? float3(0.15,0.15,0.15) : base * min(float(health)/100.0,1.0);
        }
        float2 corners[4] = {float2(-1,-1),float2(1,-1),float2(-1,1),float2(1,1)};
        for (uint i=0; i<4; i++) {
            vertices[i].position = float4((center+corners[i]*halfExtent)/4000.0,0,1);
            vertices[i].color = color;
        }
        triangles[0] = uint3(0,1,2); triangles[1] = uint3(2,1,3);
    }
}
[shader("fragment")]
float4 authorityFragment(Vertex vertex) : SV_Target0 { return float4(vertex.color,1); }
