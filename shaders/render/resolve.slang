[[vk::binding(0,0)]] Texture2D<uint2> visibilityImage;
[[vk::binding(1,0)]] Texture2D<float> depthImage;
[[vk::binding(2,0)]] Texture2D<float4> materialImages[];
[[vk::binding(3,0)]] SamplerState materialSampler;
struct ResolveVertex { float4 position : SV_Position; };
[shader("mesh")]
[numthreads(32,1,1)]
[outputtopology("triangle")]
void resolveMesh(uint lane : SV_GroupIndex, out vertices ResolveVertex vertices[3], out indices uint3 triangles[1]) {
    SetMeshOutputCounts(3,1);
    if (lane == 0) {
        vertices[0].position=float4(-1,-1,0,1); vertices[1].position=float4(3,-1,0,1); vertices[2].position=float4(-1,3,0,1);
        triangles[0]=uint3(0,1,2);
    }
}
[shader("fragment")]
float4 resolveFragment(float4 pixel : SV_Position) : SV_Target0 {
    uint2 identity = visibilityImage.Load(int3(uint2(pixel.xy),0));
    if (identity.x == 0) return float4(0.025,0.07,0.12,1);
    uint width,height;
    visibilityImage.GetDimensions(width,height);
    float2 ndc = pixel.xy / float2(width,height) * 2 - 1;
    uint instance = identity.x - 1;
    float scale = instance == 0 ? 0.5 : 1;
    float2 p = ndc / scale;
    // Perspective w is one in this qualification fixture; affine barycentrics are exact.
    float top = (0.6-p.y)/1.2;
    float right = (1-top+p.x/0.7)*0.5;
    float left = 1-top-right;
    float depth = depthImage.Load(int3(uint2(pixel.xy),0));
    float expectedDepth = instance == 0 ? 0.25 : 0.75;
    if (instance > 1 || identity.y != 0 || abs(depth-expectedDepth) > 1e-6) return float4(1,0,1,1);
    float3 tint = materialImages[NonUniformResourceIndex(instance)].SampleLevel(materialSampler,float2(right,top),0).rgb;
    return float4(float3(left,right,top)*tint,1);
}
