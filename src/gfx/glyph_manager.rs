use crossfont::{BitmapBuffer, FontDesc, Rasterize, Slant, Style, Weight};

pub struct GlyphManager {
    buffer: Vec<u8>,
    width: u32,
    height: u32,
}

impl GlyphManager {
    pub fn new() -> Self {
        // 適当なグリフを抽出してみる
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
                crossfont::Size::new(256.0),
            )
            .unwrap();
        let rasterized_glyph = rasterizer
            .get_glyph(crossfont::GlyphKey {
                character: '愛',
                font_key,
                size: crossfont::Size::new(256.0),
            })
            .unwrap();

        match rasterized_glyph.buffer {
            BitmapBuffer::Rgb(data) => Self {
                buffer: data,
                width: rasterized_glyph.width as u32,
                height: rasterized_glyph.height as u32,
            },
            BitmapBuffer::Rgba(_) => todo!(),
        }
    }

    pub async fn extract_alphabet(&mut self) {
        // どうせ使うのでアルファベットはすべて抽出
    }

    // 暫定
    #[allow(dead_code)]
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    #[allow(dead_code)]
    pub fn get_width(&self) -> u32 {
        self.width
    }

    #[allow(dead_code)]
    pub fn get_height(&self) -> u32 {
        self.height
    }
}
#[cfg(test)]
mod tests {
    use bmp::Image;

    use super::GlyphManager;

    // グリフ抽出の検証
    // リポジトリのルートに「愛」が出力される
    #[test]
    fn export() {
        let glyph_manager = GlyphManager::new();
        let buffer = glyph_manager.get_buffer();
        let mut image = Image::new(glyph_manager.get_width(), glyph_manager.get_height());

        for y in 0..glyph_manager.height as usize {
            for x in 0..glyph_manager.width as usize {
                let index = 3 * (x + (glyph_manager.width as usize) * y);
                let r = buffer[index + 0];
                let g = buffer[index + 1];
                let b = buffer[index + 2];
                image.set_pixel(x as u32, y as u32, bmp::Pixel { r, g, b });
            }
        }

        image.save("image.png").unwrap();
    }
}
