use std::collections::HashMap;

use crossfont::{BitmapBuffer, FontDesc, Rasterize, RasterizedGlyph, Slant, Style, Weight};

pub struct GlyphManager {
    rasterizer: crossfont::Rasterizer,
    font_key: crossfont::FontKey,
    rasterized_glyph_table: HashMap<char, RasterizedGlyph>,
}

impl GlyphManager {
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
            rasterized_glyph_table: HashMap::default(),
        }
    }

    #[allow(dead_code)]
    pub fn extract_alphabet(&mut self) {
        // アルファベットをあらかじめ抽出しておく
        for char_code in 'A'..='z' {
            self.extract(char_code);
        }
    }

    #[allow(dead_code)]
    pub async fn extract_alphabet_async(&mut self) {
        self.extract_alphabet();
    }

    pub fn extract(&mut self, code: char) -> bool {
        // すでに抽出済み
        if self.rasterized_glyph_table.contains_key(&code) {
            return true;
        }

        // 空白だけ特別扱い
        if code == ' ' {
            let mut buffer = Vec::default();
            buffer.resize(3 * 32 * 32, 0);
            let space = RasterizedGlyph {
                character: ' ',
                width: 32,
                height: 32,
                top: 0,
                left: 0,
                advance: (0, 0),
                buffer: BitmapBuffer::Rgb(buffer),
            };
            self.rasterized_glyph_table.insert(' ', space);
            return true;
        }

        // ラスタライズに失敗した
        let Ok(rasterized_glyph) = self.rasterizer.get_glyph(crossfont::GlyphKey {
            character: code,
            font_key: self.font_key,
            size: crossfont::Size::new(32.0),
        }) else {
            return false;
        };

        self.rasterized_glyph_table.insert(code, rasterized_glyph);
        true
    }

    pub fn get_rasterized_glyph(&self, code: char) -> &RasterizedGlyph {
        let Some(glyph) = self.rasterized_glyph_table.get(&code) else {
            return self.rasterized_glyph_table.get(&' ').unwrap();
        };

        glyph
    }

    pub fn acquire_rasterized_glyph(&mut self, code: char) -> Option<&RasterizedGlyph> {
        if !self.extract(code) {
            return None;
        }

        self.rasterized_glyph_table.get(&code)
    }
}

#[cfg(test)]
mod tests {
    use bmp::Image;
    use crossfont::BitmapBuffer;

    use super::GlyphManager;

    // グリフ抽出の検証
    // リポジトリのルートに「愛」が出力される
    #[test]
    fn export() {
        let mut glyph_manager = GlyphManager::new();
        glyph_manager.extract_alphabet();
        let (buffer, width, height) = {
            let rasterized_glyph = &glyph_manager.get_rasterized_glyph('K');
            let buffer = &rasterized_glyph.buffer;
            match buffer {
                BitmapBuffer::Rgb(buffer) => {
                    (buffer, rasterized_glyph.width, rasterized_glyph.height)
                }
                BitmapBuffer::Rgba(_) => todo!(),
            }
        };

        let mut image = Image::new(width as u32, height as u32);

        for y in 0..height as usize {
            for x in 0..width as usize {
                let index = 3 * (x + (width as usize) * y);
                let r = buffer[index];
                let g = buffer[index + 1];
                let b = buffer[index + 2];
                image.set_pixel(x as u32, y as u32, bmp::Pixel { r, g, b });
            }
        }

        image.save("image.png").unwrap();
    }
}
