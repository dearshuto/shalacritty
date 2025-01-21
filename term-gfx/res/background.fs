#version 450

layout(location = 0) out vec4 o_Color;
layout(location = 0) in vec2 v_TexCoord;

void main()
{
    o_Color = vec4(v_TexCoord, 0.0, 1.0);
}
