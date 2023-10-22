use std::collections::HashMap;

use crossfont::{FontDesc, Rasterize, RasterizedGlyph, Slant, Style, Weight};

pub struct GlyphManager {
    rasterized_glyph_table: HashMap<char, RasterizedGlyph>,
}

impl GlyphManager {
    pub fn new() -> Self {
        Self {
            rasterized_glyph_table: HashMap::default(),
        }
    }

    pub fn extract_alphabet(&mut self) {
        let mut rasterizer = crossfont::Rasterizer::new(1.0).unwrap();
        let font_key = rasterizer
            .load_font(
                &FontDesc::new(
                    "Menlo",
                    Style::Description {
                        slant: Slant::Normal,
                        weight: Weight::Normal,
                    },
                ),
                crossfont::Size::new(32.0),
            )
            .unwrap();

        for char_code in ('a'..='z').chain('A'..'Z') {
            let rasterized_glyph = rasterizer
                .get_glyph(crossfont::GlyphKey {
                    character: char_code,
                    font_key,
                    size: crossfont::Size::new(32.0),
                })
                .unwrap();
            self.rasterized_glyph_table
                .insert(char_code, rasterized_glyph);
        }
    }

    pub async fn extract_alphabet_async(&mut self) {
        self.extract_alphabet();
    }

    pub fn get_rasterized_glyph(&self, code: char) -> &RasterizedGlyph {
        self.rasterized_glyph_table.get(&code).unwrap()
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
                let r = buffer[index + 0];
                let g = buffer[index + 1];
                let b = buffer[index + 2];
                image.set_pixel(x as u32, y as u32, bmp::Pixel { r, g, b });
            }
        }

        image.save("image.png").unwrap();
    }
}
