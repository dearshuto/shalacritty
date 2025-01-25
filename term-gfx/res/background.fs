#version 450

layout(location = 0) out vec4 o_Color;
layout(location = 0) in vec2 v_TexCoord;

// 0 番目は頂点シェーダーで使用ずみ
layout(binding = 1) uniform texture2D u_GlyphTexture;
layout(binding = 2) uniform sampler u_GlyphSampler;

void main()
{
    o_Color = vec4(v_TexCoord, 0.0, 1.0);
}
