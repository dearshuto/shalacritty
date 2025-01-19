#version 450

layout(location = 0) out vec4 v_Color;
layout(location = 0) in vec2 i_Position;

struct CharacterData
{
    vec4 transform[2];
    vec4 foreGroundColor;
    vec2 uv0;
    vec2 uv1;
};

layout(std430, binding = 0) readonly buffer CharacterDataBuffer
{
    CharacterData u_CharacterDatas[];
};

void main() {
    CharacterData data = u_CharacterDatas[gl_InstanceIndex];

    // 色
    v_Color = data.foreGroundColor;

    // 座標変換
    vec2 position = vec2(
            dot(data.transform[0], vec4(i_Position, 0.0, 1.0)),
            dot(data.transform[1], vec4(i_Position, 0.0, 1.0)));
    gl_Position = vec4(position, 0.0, 1.0);
}
