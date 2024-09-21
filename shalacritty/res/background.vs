#version 450

layout (location = 0) out vec2 v_Uv;
layout (location = 0) in vec2 i_Position;

layout (binding = 0) uniform View {
    vec4 u_ImageTransform[2];
};

void main()
{
    gl_Position = vec4(i_Position, 0.0, 1.0);

    vec3 uv = vec3((i_Position + 1.0) / 2.0, 1.0);
    v_Uv = vec2(dot(u_ImageTransform[0].xyz, uv), dot(u_ImageTransform[1].xyz, uv));
}
