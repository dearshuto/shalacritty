#version 450

layout (location = 0) in vec2 i_Position;

layout (binding = 0) uniform View {
    vec4 u_Transform[16];
};

void main()
{
    int index = gl_InstanceIndex;
    vec2 position = vec2(
        dot(u_Transform[index + 0].xyz, vec3(i_Position, 1.0)),
        dot(u_Transform[index + 1].xyz, vec3(i_Position, 1.0))
        );
    gl_Position = vec4(position, 0.0, 1.0);
}
