use crossfont::{FontDesc, Rasterize, RasterizedGlyph, Slant, Style, Weight};

pub struct FontEngine {
    rasterizer: crossfont::Rasterizer,
    font_key: crossfont::FontKey,
}

impl FontEngine {
    pub fn new_with_font_size(font_size: f32) -> Self {
        let mut rasterizer = crossfont::Rasterizer::new().unwrap();
        let font_key = Self::create_font_key(&mut rasterizer, font_size);
        Self {
            rasterizer,
            font_key,
        }
    }

    pub fn rasterize(
        &mut self,
        code: char,
        size: f32,
    ) -> Result<RasterizedGlyph, crossfont::Error> {
        // ここで渡している font_size はライブラリ内部で使用されてないっぽい
        // しかし要求されてる以上はフォントサイズを指定している
        self.rasterizer.get_glyph(crossfont::GlyphKey {
            character: code,
            font_key: self.font_key,
            size: crossfont::Size::new(size),
        })
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_key = Self::create_font_key(&mut self.rasterizer, font_size);
    }

    fn create_font_key(
        rasterizer: &mut crossfont::Rasterizer,
        font_size: f32,
    ) -> crossfont::FontKey {
        rasterizer
            .load_font(
                &FontDesc::new(
                    #[cfg(not(any(target_os = "macos", windows)))]
                    "monospace",
                    #[cfg(target_os = "macos")]
                    "Menlo",
                    #[cfg(target_os = "windows")]
                    "Consolas",
                    Style::Description {
                        slant: Slant::Normal,
                        weight: Weight::Normal,
                    },
                ),
                crossfont::Size::new(font_size),
            )
            .unwrap()
    }
}
