#version 450

layout(location = 0) out vec2 v_Color;
layout(location = 0) in vec2 i_Position;

void main() {
    v_Color = (i_Position + vec2(1.0)) / 2.0;

    if (gl_InstanceIndex == 0)
    {
        gl_Position = vec4(0.5 * i_Position + vec2(0.5, 0.0), 0.0, 1.0);
    }
    else
    {
        gl_Position = vec4(0.5 * i_Position - vec2(0.5, 0.0), 0.0, 1.0);
    }
}
