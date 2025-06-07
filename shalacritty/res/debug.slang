struct VertexOutput
{
  float4 position: SV_Position;
}

[shader("vertex")]
VertexOutput main_vs(int vertexId: SV_VertexID)
{
  VertexOutput vertexOutput;

  if (vertexId == 0)
  {
    vertexOutput.position = float4(0.0, 1.0, 0.0, 1.0);
  } else if (vertexId == 1) {
    vertexOutput.position = float4(-1.0, -1.0, 0.0, 1.0);
  } else {
    vertexOutput.position = float4(1.0, -1.0, 0.0, 1.0);
  }
  
  return vertexOutput;
}

[shader("fragment")]
float4 main_fs()
{
  return float4(1.0);
}
