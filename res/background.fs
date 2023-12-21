#version 450

layout (location = 0) out vec4 o_Color;
layout (location = 0) in vec2 v_Uv;

layout (binding = 1) uniform texture2D u_BackgroundTexture;
layout (binding = 2) uniform sampler u_Sampler;

layout (binding = 3) uniform Material {
    float u_AlphaEnhance;
};

void main()
{
    vec4 color = texture(sampler2D(u_BackgroundTexture, u_Sampler), v_Uv);
    o_Color = color;

    // 定数バッファーで指定できるようにする
    o_Color.a *= u_AlphaEnhance;
}
