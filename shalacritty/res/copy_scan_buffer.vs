#version 450

layout (location = 0) out vec2 v_Uv;

layout (binding = 0) uniform View {
    vec2 u_Scale;
};

// TODO: 頂点バッファー対応
void main()
{
    if (gl_VertexIndex == 0 || gl_VertexIndex == 3)
    {
        gl_Position = vec4(-1.0, 1.0, 0.0, 1.0);
    }
    else if (gl_VertexIndex == 1)
    {
        gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
    }
    else if (gl_VertexIndex == 2 || gl_VertexIndex == 4)
    {
        gl_Position = vec4(1.0, -1.0, 0.0, 1.0);
    }
    else if (gl_VertexIndex == 5)
    {
        gl_Position = vec4(1.0, 1.0, 0.0, 1.0);
    }

    v_Uv = u_Scale *  ((gl_Position + 1.0) / 2.0).xy;
}
