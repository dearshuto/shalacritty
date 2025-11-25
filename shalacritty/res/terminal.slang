struct BackgroundView {
  float4 transform0;
  float4 transform1;
};

ConstantBuffer<BackgroundView> g_BackgroundView;

struct PositionCoordOutput {
  float4 position : SV_Position;
  float2 uv : TEXTOORD0;
};

[shader("vertex")]
float4 main_vs(float2 position) : SV_Position {
  return float4(position, 0.0, 1.0);
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
float4 main_fs() { return float4(1.0); }

[shader("fragment")]
float4 background_fs(float2 uv: TEXTOORD0) : SV_Target {
  float4 color = g_BackgroundTexture.Sample(g_SamplerState, uv);
  return color;
}

// ============================================================================

struct RectView {
  float4 transform[16];
}

ConstantBuffer<RectView> g_RectView;

[shader("vertex")]
float4 rect_vs(int instanceId: SV_InstanceID, float2 position) : SV_Position {
  float2 outPosition = float2(
      dot(g_RectView.transform[instanceId + 0].xyz, float3(position, 1.0)),
      dot(g_RectView.transform[instanceId + 1].xyz, float3(position, 1.0)));
  return float4(outPosition, 0.0, 1.0);
}

[shader("fragment")]
float4 rect_fs() : SV_Target { return float4(1.0); }

// ============================================================================

struct CharacterData {
  float4 transform[2];
  float4 foreGroundColor;
  float4 uv01;
}

StructuredBuffer<CharacterData> g_CharacterDataBuffer;

struct CharacterVertexOutput {
  float4 position : SV_Position;
  nointerpolation float4 fgColor : COLOR0;
  float2 uv : TEXCOORD1;
}

[shader("vertex")]
CharacterVertexOutput character_vs(int vertexId: SV_VertexID,
                                   int instanceId: SV_InstanceID,
                                   float2 position) {
  CharacterData data = g_CharacterDataBuffer[instanceId];

  CharacterVertexOutput output;
  output.position =
      float4(dot(data.transform[0].xyz, float3(position, 1.0)),
             dot(data.transform[1].xyz, float3(position, 1.0)), 0.0, 1.0);
  output.fgColor = data.foreGroundColor;

  // TODO: 条件分岐を消したい
  // TODO: UV の上下反転をちゃんと考えたい
  // 0 - 3
  // |   |
  // 1 - 2
  if (vertexId == 1) {
    output.uv = data.uv01.xy;
  } else if (vertexId == 0) {
    output.uv.x = data.uv01.x;
    output.uv.y = data.uv01.w;
  } else if (vertexId == 3) {
    output.uv = data.uv01.zw;
  } else if (vertexId == 2) {
    output.uv.x = data.uv01.z;
    output.uv.y = data.uv01.y;
  }

  return output;
}

[[vk::binding(0)]]
SamplerState g_GlyphSamplerState : register(s0);

[[vk::binding(1)]]
Texture2D<float> g_GlyphTexture : register(t0);

[shader("fragment")]
float4 character_fs(float4 color: COLOR0, float2 uv: TEXCOORD1) {
  float alpha = g_GlyphTexture.Sample(g_SamplerState, uv);
  return float4(color.x, color.y, color.z, alpha);
}

struct CharacterDataTentative {
  // 0 - 2
  // |   |
  // 1 - 3
  int vertexId : SV_VertexID;

  // 頂点ごとに割り当たる値
  [[vk::location(0)]]
  float2 position;

  // インスタンスごとに割り当たる値
  [[vk::location(1)]]
  float4 transform0;
  [[vk::location(2)]]
  float4 transform1;
  [[vk::location(3)]]
  float4 fgColor;
  [[vk::location(4)]]
  float4 uv01;
}

[shader("vertex")]
CharacterVertexOutput character_vs_tentative(CharacterDataTentative data) {
  CharacterVertexOutput output;
  output.position =
      float4(dot(data.transform0.xyz, float3(data.position, 1.0)),
             dot(data.transform1.xyz, float3(data.position, 1.0)), 0.0, 1.0);
  output.fgColor = data.fgColor;
  float x[2] = { data.uv01.x, data.uv01.z };
  float y[2] = { data.uv01.y, data.uv01.w };
  output.uv.x = x[data.vertexId / 2];
  output.uv.y = y[data.vertexId % 2];

  return output;
}

[shader("fragment")]
float4 character_fs_tentative(float4 fgColor: COLOR0, float2 uv: TEXCOORD1)
    : SV_Target {
  float alpha = g_GlyphTexture.Sample(g_GlyphSamplerState, uv);
  return float4(uv.x, uv.y, alpha, 1.0);
}

// ============================================================================

struct CopyView {
  float4 scale;
}

ConstantBuffer<CopyView> g_CopyView;

[shader("vertex")]
PositionCoordOutput copy_vs(int vertexId: SV_VertexID) {
  PositionCoordOutput output;

  if (vertexId == 0 || vertexId == 3) {
    output.position = float4(-1.0, 1.0, 0.0, 1.0);
  } else if (vertexId == 1) {
    output.position = float4(-1.0, -1.0, 0.0, 1.0);
  } else if (vertexId == 2 || vertexId == 4) {
    output.position = float4(1.0, -1.0, 0.0, 1.0);
  } else if (vertexId == 5) {
    output.position = float4(1.0, 1.0, 0.0, 1.0);
  }

  output.uv = g_CopyView.scale.xy * ((output.position + 1.0) / 2.0).xy;

  return output;
}

Texture2D<float4> g_Texture;

[shader("fragment")]
float4 copy_fs(float2 uv: TEXCOOD0) : SV_Target {
  return g_Texture.Sample(g_SamplerState, uv);
}
