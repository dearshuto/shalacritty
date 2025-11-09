use crossfont::RasterizedGlyph;

#[derive(Debug)]
pub struct GlyphRange {}

impl GlyphRange {
    pub fn lower_left(&self) -> [f32; 2] {
        [0.0, 0.0]
    }

    pub fn upper_right(&self) -> [f32; 2] {
        [0.0, 0.0]
    }
}

pub struct GlyphTexture {
    #[allow(unused)]
    device: ash::Device,
    // 過去にラスタライズした文字
    // glyph_cache: HashSet<char>,
}

impl GlyphTexture {
    pub fn new(device: ash::Device) -> Self {
        Self { device }
    }

    pub fn range(&self, _code: char) -> GlyphRange {
        GlyphRange {}
    }

    pub fn write(&self, _glyphs: &[RasterizedGlyph]) {}
}
