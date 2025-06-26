struct BackgroundView
{
  float4 transform0;
  float4 transform1;
};

ConstantBuffer<BackgroundView> g_BackgroundView;


struct PositionCoordOutput
{
  float4 position : SV_Position;
  float2 uv : TEXTOORD0;
};

[shader("vertex")]
float4 main_vs(int vertexId : SV_VertexID) : SV_Position {
  if (vertexId == 0)
  {
    return float4(0.0, 1.0, 0.0, 1.0);
  } else if (vertexId == 1) {
    return float4(-1.0, -1.0, 0.0, 1.0);
  } else {
    return float4(1.0, -1.0, 0.0, 1.0);
  }
}

[shader("vertex")]
PositionCoordOutput background_vs(float2 position) {
  PositionCoordOutput output;
  
   // position は [-1.0, 1.0] を想定
  float2 uv = 0.5 * (position + 1.0);
  output.uv.x = dot(g_BackgroundView.transform0.xyz, float3(uv, 1.0));
  output.uv.y = dot(g_BackgroundView.transform1.xyz, float3(uv, 1.0));
  output.position = float4(position, 0.0, 1.0);

 return output;
}


// ============================================================================
Texture2D<float4> g_BackgroundTexture;
SamplerState g_SamplerState;


[shader("fragment")]
float4 main_fs()
{
  return float4(1.0);
}

[shader("fragment")]
float4 background_fs(float2 uv : TEXTOORD0) : SV_Target
{
  float4 color = g_BackgroundTexture.Sample(g_SamplerState, uv);
  return color;
}


// ============================================================================

struct RectView
{
  float4 transform[16];
}

ConstantBuffer<RectView> g_RectView;

[shader("vertex")]
float4 rect_vs(int instanceId: SV_InstanceID, float2 position) : SV_Position {
  float2 outPosition = float2(
    dot(g_RectView.transform[instanceId + 0].xyz, float3(position, 1.0)),
    dot(g_RectView.transform[instanceId + 1].xyz, float3(position, 1.0))
  );
  return float4(outPosition, 0.0, 1.0);
}

[shader("fragment")]
float4 rect_fs() : SV_Target {
  return float4(1.0);
}

// ============================================================================

struct CharacterData
{
  float4 transform[2];
  float4 foreGroundColor;
  float2 uv0;
  float2 uv1;
}

StructuredBuffer<CharacterData> g_CharacterDataBuffer;

struct CharacterVertexOutput
{
  float4 position : SV_Position;
  float4 fgColor : COLOR0;
  float2 uv : TEXCOORD1;
}

[shader("vertex")]
CharacterVertexOutput character_vs(int vertexId : SV_VertexID,  int instanceId : SV_InstanceID, float2 position)
{
  CharacterData data = g_CharacterDataBuffer[instanceId];

  CharacterVertexOutput output;
  output.position = float4(
          dot(data.transform[0].xyz, float3(position, 1.0)),
          dot(data.transform[1].xyz, float3(position, 1.0)),
          0.0,
          1.0);
  output.fgColor = data.foreGroundColor;

  // TODO: 条件分岐を消したい
  // TODO: UV の上下反転をちゃんと考えたい
  // 0 - 3
  // |   |
  // 1 - 2
  if (vertexId == 1)
  {
    output.uv = data.uv0;
  }
  else if (vertexId == 0)
  {
    output.uv.x = data.uv0.x;
    output.uv.y = data.uv1.y;
  }
  else if (vertexId == 3)
  {
    output.uv = data.uv1;
  }
  else if (vertexId == 2)
  {
    output.uv.x = data.uv1.x;
    output.uv.y = data.uv0.y;
  }
  
  return output;
}

Texture2D<float> g_GlyphTexture;

[shader("fragment")]
float4 character_fs(float4 color : COLOR0,  float2 uv : TEXCOORD1)
{
  float alpha = g_GlyphTexture.Sample(g_SamplerState, uv);
  return float4(color.x, color.y, color.z, alpha);
}


// ============================================================================

struct CopyView
{
  float4 scale;
}

ConstantBuffer<CopyView> g_CopyView;

[shader("vertex")]
PositionCoordOutput copy_vs(int vertexId: SV_VertexID)
{
  PositionCoordOutput output;

  if (vertexId == 0 || vertexId == 3)
  {
    output.position = float4(-1.0, 1.0, 0.0, 1.0);
  }
  else if (vertexId == 1)
  {
    output.position = float4(-1.0, -1.0, 0.0, 1.0);
  }
  else if (vertexId == 2 || vertexId == 4)
  {
    output.position = float4(1.0, -1.0, 0.0, 1.0);
  }
  else if (vertexId == 5)
  {
    output.position = float4(1.0, 1.0, 0.0, 1.0);
  }

  output.uv = g_CopyView.scale.xy * ((output.position + 1.0) / 2.0).xy;
  
  return output;
}

Texture2D<float4> g_Texture;

[shader("fragment")]
float4 copy_fs(float2 uv: TEXCOOD0) : SV_Target
{
  return g_Texture.Sample(g_SamplerState, uv);
  
}
