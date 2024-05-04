use crossfont::{FontDesc, Rasterize, RasterizedGlyph, Slant, Style, Weight};

pub struct FontEngine {
    rasterizer: crossfont::Rasterizer,
    font_key: crossfont::FontKey,
}

impl FontEngine {
    pub fn new() -> Self {
        let mut rasterizer = crossfont::Rasterizer::new().unwrap();
        let font_key = rasterizer
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
                crossfont::Size::new(32.0),
            )
            .unwrap();
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
        self.rasterizer.get_glyph(crossfont::GlyphKey {
            character: code,
            font_key: self.font_key,
            size: crossfont::Size::new(size),
        })
    }
}
