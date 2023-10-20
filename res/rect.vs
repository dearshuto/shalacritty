#version 450

layout (location = 0) out vec2 v_Uv;
layout (location = 0) in vec2 i_Position;

struct CharacterData
{
    vec4 transform[2];
    vec2 uv0;
    vec2 uv1;
};

layout(std430, binding = 0) readonly buffer CharacterDataBuffer
{
    CharacterData u_CharacterDatas[];
};

void main()
{
    vec2 position = vec2(
        dot(u_CharacterDatas[gl_InstanceIndex].transform[0].xyz, vec3(i_Position, 1.0)),
        dot(u_CharacterDatas[gl_InstanceIndex].transform[1].xyz, vec3(i_Position, 1.0))
        );
    gl_Position = vec4(position, 0.0, 1.0);

    // 位置は -0.5~0.5
    v_Uv = (i_Position + 0.5);
}
