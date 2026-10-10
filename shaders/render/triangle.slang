// M0-D qualification: all geometry is generated in the mesh shader.
struct Vertex {
    float4 position : SV_Position;
    [[vk::location(0)]] float3 color : COLOR0;
};

[shader("mesh")]
[numthreads(32, 1, 1)]
[outputtopology("triangle")]
void meshMain(uint lane : SV_GroupIndex,
              out vertices Vertex vertices[3],
              out indices uint3 triangles[1]) {
    SetMeshOutputCounts(3, 1);
    if (lane == 0) {
        vertices[0].position = float4(-0.7, 0.6, 0.0, 1.0);
        vertices[0].color = float3(1.0, 0.0, 0.0);
        vertices[1].position = float4(0.7, 0.6, 0.0, 1.0);
        vertices[1].color = float3(0.0, 1.0, 0.0);
        vertices[2].position = float4(0.0, -0.6, 0.0, 1.0);
        vertices[2].color = float3(0.0, 0.0, 1.0);
        triangles[0] = uint3(0, 1, 2);
    }
}

[shader("fragment")]
float4 fragmentMain(Vertex vertex) : SV_Target0 {
    return float4(vertex.color, 1.0);
}
