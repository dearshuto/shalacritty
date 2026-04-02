#[repr(C)]
pub struct CharacterData {
    pub transform0: [f32; 4],
    pub transform1: [f32; 4],
    pub fg_color: [f32; 4],
    pub uv01: [f32; 4],
}
