// Tiny benchmark-only 5x7 debug font. General engine UI/text shaping stays separate.
struct Hud { uint4 perf; uint4 scene; };
[[vk::push_constant]] ConstantBuffer<Hud> hud;
static const uint2 hudFont[38]={
uint2(0x239ace2eu,14u),
uint2(0x08421184u,14u),
uint2(0x1041062eu,31u),
uint2(0x0217043eu,30u),
uint2(0x05f928c2u,2u),
uint2(0x021f421fu,30u),
uint2(0x231f420eu,14u),
uint2(0x1082083fu,8u),
uint2(0x2317462eu,14u),
uint2(0x0217c62eu,14u),
uint2(0x231fc62eu,17u),
uint2(0x231f463eu,30u),
uint2(0x2108420fu,15u),
uint2(0x2318c63eu,30u),
uint2(0x210f421fu,31u),
uint2(0x210f421fu,16u),
uint2(0x231bc20fu,15u),
uint2(0x231fc631u,17u),
uint2(0x0842108eu,14u),
uint2(0x23108421u,14u),
uint2(0x254c5251u,17u),
uint2(0x21084210u,31u),
uint2(0x231ad771u,17u),
uint2(0x2319d731u,17u),
uint2(0x2318c62eu,14u),
uint2(0x210f463eu,16u),
uint2(0x2558c62eu,13u),
uint2(0x254f463eu,17u),
uint2(0x0217420fu,30u),
uint2(0x0842109fu,4u),
uint2(0x2318c631u,14u),
uint2(0x1518c631u,4u),
uint2(0x2b5ac631u,10u),
uint2(0x22a22a31u,17u),
uint2(0x08422a31u,4u),
uint2(0x2082083fu,31u),
uint2(0x08000000u,4u),
uint2(0x20820821u,16u)
};
static const uint hudPrefixes[9][16]={
{84,72,69,73,65,32,66,69,78,67,72,77,65,82,75,32},
{70,80,83,32,32,32,32,32,32,32,32,32,32,32,32,32},
{67,80,85,32,77,83,32,32,32,32,32,32,32,32,32,32},
{71,80,85,32,77,83,32,32,32,32,32,32,32,32,32,32},
{86,73,83,73,66,76,69,32,32,32,32,32,32,32,32,32},
{80,82,79,88,73,69,83,32,32,32,32,32,32,32,32,32},
{78,69,65,82,32,32,32,32,32,32,32,32,32,32,32,32},
{65,76,76,79,67,32,77,73,66,32,32,32,32,32,32,32},
{80,72,65,83,69,32,32,32,32,32,32,32,32,32,32,32}
};
static const uint hudPrefixLength[9]={15,4,7,7,8,8,5,10,6};
uint hudDigits(uint v) { uint count=1;while(v>=10) {v/=10;++count;}return count; }
uint hudPow(uint n) {uint v=1;for(uint i=0;i<n;++i)v*=10;return v;}
uint hudUInt(uint value,uint col) {uint n=hudDigits(value);if(col>=n)return 32;return 48+(value/hudPow(n-col-1))%10;}
uint hudDecimal(uint value,uint precision,uint col) {
    uint divisor=hudPow(precision),n=hudDigits(value/divisor);
    if(col<n)return hudUInt(value/divisor,col);
    if(col==n)return 46;
    if(col<=n+precision)return 48+(value/hudPow(precision-(col-n)))%10;
    return 32;
}
uint hudCharacter(uint line,uint column) {
    if(line>=9)return 32;
    uint prefix=hudPrefixLength[line];if(column<prefix)return hudPrefixes[line][column];
    uint col=column-prefix;
    if(line==0)return 32;
    if(line==1)return hudDecimal(hud.perf.x,1,col);
    if(line==2)return hudDecimal(hud.perf.y,3,col);
    if(line==3)return hudDecimal(hud.perf.z,3,col);
    if(line==4 || line==5) {
        uint value=benchmarkInstances.Load(line==4 ? 0:28),n=hudDigits(value);
        if(col<n)return hudUInt(value,col);if(col==n)return 47;
        return hudUInt(benchmarkInstances.Load(16)-(line==5 ? 448u:0u),col-n-1);
    }
    if(line==6) {
        uint value=benchmarkInstances.Load(20),n=hudDigits(value);
        if(col<n)return hudUInt(value,col);
        uint separator[5]={32,70,65,82,32};
        if(col<n+5)return separator[col-n];return hudUInt(benchmarkInstances.Load(24),col-n-5);
    }
    if(line==7)return hudDecimal(hud.scene.z,1,col);
    if(col==0)return hudUInt(hud.scene.x,0);
    uint separator[3]={32,84,32};if(col<4)return separator[col-1];
    return hudDecimal(hud.scene.y,1,col-4);
}
float4 hudPixel(uint2 pixel) {
    if(pixel.x<14 || pixel.y<14)return float4(0.04,0.5,0.4,1);
    if(pixel.x<24 || pixel.y<24)return float4(0.012,0.025,0.04,1);
    uint2 local=(pixel-uint2(24,24))/3;
    uint line=local.y/8,column=local.x/6,x=local.x%6,y=local.y%8;
    uint c=hudCharacter(line,column);
    uint index=c>=48 && c<=57 ? c-48:c>=65 && c<=90 ? c-65+10:c==46 ? 36:c==47 ? 37:38;
    bool lit=false;
    if(index<38 && x<5 && y<7) {
        uint2 bits=hudFont[index];uint row=y<6 ? (bits.x>>(y*5))&31:bits.y;
        lit=(row&(1u<<(4-x)))!=0;
    }
    return lit ? float4(0.7,0.92,0.76,1):float4(0.012,0.025,0.04,1);
}
