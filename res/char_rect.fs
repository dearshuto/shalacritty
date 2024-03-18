#version 450

layout (location = 0) out vec4 o_Color;
layout (location = 0) in vec2 v_Uv;
layout (location = 1) in vec4 v_ForeGroundColor;
layout (location = 2) in vec4 v_BackGroundColor;

layout (binding = 1) uniform texture2D u_GlyphTexture;
layout (binding = 2) uniform sampler u_GlyphSampler;

void main()
{
    // グリフをアルファで抜く
    vec4 alpha = texture(sampler2D(u_GlyphTexture, u_GlyphSampler), v_Uv);

    // 文字色と背景色をブレンド
    vec3 color = mix(v_BackGroundColor.xyz, v_ForeGroundColor.xyz, alpha.x);
    o_Color = vec4(color, 1.0);
}
