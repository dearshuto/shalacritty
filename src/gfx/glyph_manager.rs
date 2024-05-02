use std::collections::HashMap;

use crossfont::{BitmapBuffer, RasterizedGlyph};

use super::detail::FontEngine;

use super::{
    content_plotter::GlyphTexturePatch,
    detail::{CharacterData, GlyphWriter, IGlyphManager},
};

pub struct CharacterClipRect {
    pub uv_begin: [f32; 2],
    pub uv_end: [f32; 2],
}

impl From<CharacterData> for CharacterClipRect {
    fn from(value: CharacterData) -> Self {
        CharacterClipRect {
            uv_begin: value.uv_begin,
            uv_end: value.uv_end,
        }
    }
}

struct GlyphTable {
    rasterized_glyph_table: HashMap<char, RasterizedGlyph>,
}

impl IGlyphManager for GlyphTable {
    fn acquire_rasterized_glyph(&self, code: char) -> Option<&RasterizedGlyph> {
        self.rasterized_glyph_table.get(&code)
    }
}

pub struct GlyphManager {
    font_engine: FontEngine,
    glyph_writer: GlyphWriter,
    glyph_table: GlyphTable,
}

impl GlyphManager {
    pub fn new() -> Self {
        Self {
            font_engine: FontEngine::new(),
            glyph_table: GlyphTable {
                rasterized_glyph_table: HashMap::new(),
            },
            glyph_writer: GlyphWriter::new(),
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

    pub fn extract(&mut self, code: char) -> Option<GlyphTexturePatch> {
        // すでに抽出済み
        if self.glyph_table.rasterized_glyph_table.contains_key(&code) {
            return None;
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
            self.glyph_table.rasterized_glyph_table.insert(' ', space);
            let mut glyph_texture_patches = self
                .glyph_writer
                .execute([' '].into_iter(), &self.glyph_table);

            return Some(glyph_texture_patches.remove(0).into());
        }

        // ラスタライズに失敗した
        let Ok(rasterized_glyph) = self.font_engine.rasterize(code, 32.0) else {
            return None;
        };

        self.glyph_table
            .rasterized_glyph_table
            .insert(code, rasterized_glyph);

        let mut glyph_texture_patches = self
            .glyph_writer
            .execute([code].into_iter(), &self.glyph_table);

        Some(glyph_texture_patches.remove(0).into())
    }

    pub fn extract_range<TIterator>(
        &mut self,
        codes: TIterator,
    ) -> impl Iterator<Item = GlyphTexturePatch>
    where
        TIterator: Iterator<Item = char>,
    {
        let mut glyph_texture_patches = Vec::default();
        for code in codes {
            let Some(patch) = self.extract(code) else {
                continue;
            };

            glyph_texture_patches.push(patch);
        }

        glyph_texture_patches.into_iter()
    }

    // 実運用を考えたらノーチェックでグリフを取得する関数は不要かも？
    pub fn get_rasterized_glyph(&self, code: char) -> &RasterizedGlyph {
        self.acquire_rasterized_glyph(code).unwrap()
    }

    pub fn acquire_rasterized_glyph(&self, code: char) -> Option<&RasterizedGlyph> {
        self.glyph_table.rasterized_glyph_table.get(&code)
    }

    pub fn get_clip_rect(&self, code: char) -> CharacterClipRect {
        self.glyph_writer.get_clip_rect(code).into()
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
