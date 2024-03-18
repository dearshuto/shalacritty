#version 450

layout (location = 0) out vec2 v_Uv;
layout (location = 1) out vec4 v_ForeGroundColor;
layout (location = 2) out vec4 v_BackGroundColor;
layout (location = 0) in vec2 i_Position;

struct CharacterData
{
    vec4 transform[2];
    vec4 foreGroundColor;
    vec4 backGroundColor;
    vec2 uv0;
    vec2 uv1;
};

layout(std430, binding = 0) readonly buffer CharacterDataBuffer
{
    CharacterData u_CharacterDatas[];
};

void main()
{
    CharacterData characterData = u_CharacterDatas[gl_InstanceIndex];
    vec2 position = vec2(
        dot(characterData.transform[0].xyz, vec3(i_Position, 1.0)),
        dot(characterData.transform[1].xyz, vec3(i_Position, 1.0))
        );
    gl_Position = vec4(position, 0.0, 1.0);
    v_ForeGroundColor = characterData.foreGroundColor;
    v_BackGroundColor = characterData.backGroundColor;

    // TODO: 条件分岐を消したい
    // TODO: UV の上下反転をちゃんと考えたい
    // 0 - 3
    // |   |
    // 1 - 2
    if (gl_VertexIndex == 1)
    {
        v_Uv = characterData.uv0;
    }
    else if (gl_VertexIndex == 0)
    {
        v_Uv.x = characterData.uv0.x;
        v_Uv.y = characterData.uv1.y;
    }
    else if (gl_VertexIndex == 3)
    {
        v_Uv = characterData.uv1;
    }
    else if (gl_VertexIndex == 2)
    {
        v_Uv.x = characterData.uv1.x;
        v_Uv.y = characterData.uv0.y;
    }
}
