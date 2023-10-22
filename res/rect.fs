#version 450

layout (location = 0) out vec4 o_Color;
layout (location = 0) in vec2 v_Uv;

layout (binding = 1) uniform texture2D u_GlyphTexture;
layout (binding = 2) uniform sampler u_GlyphSampler;

void main()
{
    // グリフをアルファで抜く
    // TODO: 色に対応する
    vec4 alpha = texture(sampler2D(u_GlyphTexture, u_GlyphSampler), v_Uv);
    o_Color = vec4(vec3(1.0), alpha);
}
