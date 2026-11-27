// Specific fallback: Slang 2026.19 fragment PerPrimitiveEXT input is unqualified.
#version 460
#extension GL_EXT_mesh_shader : require
layout(location=0) perprimitiveEXT flat in uvec2 primitiveIdentity;
layout(location=0) out uvec2 identity;
void main() { identity = primitiveIdentity; }
