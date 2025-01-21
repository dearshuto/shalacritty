#version 450

layout(location = 0) out vec2 v_TexCoord;
layout(location = 0) in vec2 i_Position;

layout(std140, binding = 0) uniform View
{
    vec4 u_TexCoordTransform[2];
};

void main()
{
    // 一辺の長さが 1 の正方形を想定
    // 画面全体を
    gl_Position = vec4(2.0 * i_Position, 0.0, 1.0);

    vec3 uv = vec3(i_Position, 1.0);
    v_TexCoord = vec2(
            dot(u_TexCoordTransform[0].xyz, uv),
            dot(u_TexCoordTransform[1].xyz, uv));
}
