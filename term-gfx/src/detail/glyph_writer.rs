use std::collections::HashMap;

use crossfont::{BitmapBuffer, RasterizedGlyph};

#[derive(Clone, Copy)]
struct CharacterCache {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct CharacterData {
    pub uv_begin: [f32; 2],
    pub uv_end: [f32; 2],
}

pub struct GlyphImagePatch {
    offset_x: u32,
    offset_y: u32,
    width: u32,
    height: u32,
    pixel_data: Vec<u8>,
}

impl GlyphImagePatch {
    pub fn offset_x(&self) -> u32 {
        self.offset_x
    }

    pub fn offset_y(&self) -> u32 {
        self.offset_y
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixel_data
    }
}

pub trait IGlyphManager {
    fn acquire_rasterized_glyph(&self, code: char) -> Option<&RasterizedGlyph>;
}

pub struct GlyphWriter {
    image_width: u32,
    image_height: u32,

    // キャッシュ
    character_data: HashMap<char, CharacterCache>,

    // 現在どこまでテクスチャーを利用しているか
    current_x: u32,
    current_y: u32,

    buffer: Vec<u8>,
}

impl GlyphWriter {
    pub fn new() -> Self {
        let mut buffer = Vec::default();
        buffer.resize(4096 * 4096, 0);

        Self {
            image_width: 64 * 64, /*64 ピクセルを 64 文字で 4096 ピクセル*/
            image_height: 64 * 64,
            character_data: HashMap::default(),
            current_x: 0,
            current_y: 0,
            buffer,
        }
    }

    pub fn execute<T, U: IGlyphManager>(
        &mut self,
        codes: T,
        glyph_manager: &U,
    ) -> Vec<GlyphImagePatch>
    where
        T: Iterator<Item = char>,
    {
        let diff_items = codes
            .filter_map(|code| {
                if self.character_data.contains_key(&code) {
                    return None;
                };

                // 空白はグリフが存在しないので特別扱い
                if code == ' ' {
                    return Some((
                        ' ',
                        CharacterCache {
                            x: 4000,
                            y: 4000,
                            width: 32,
                            height: 32,
                        },
                    ));
                }

                let glyph = glyph_manager.acquire_rasterized_glyph(code)?;

                let offset_x = self.current_x * 64;
                let offset_y = self.current_y * 64;
                let character_data = CharacterCache {
                    x: offset_x,
                    y: offset_y,
                    width: glyph.width as u32,
                    height: glyph.height as u32,
                };
                // x がはじまで到達したら、y は次の行に移動して x は先頭に戻る
                self.current_y += (self.current_x + 1) / 64;
                self.current_x = (self.current_x + 1) % 64;
                Some((code, character_data))
            })
            .collect::<Vec<(char, CharacterCache)>>();

        // キャッシュに反映
        for (code, character_cache) in &diff_items {
            self.character_data.insert(*code, *character_cache);
        }

        // グリフのパッチ
        let glyph_image_patches = diff_items
            .iter()
            .filter_map(|(code, character_cache)| {
                // グリフを取得
                let glyph = glyph_manager.acquire_rasterized_glyph(*code)?;
                let buffer = match &glyph.buffer {
                    BitmapBuffer::Rgb(buffer) => {
                        buffer.chunks(3).map(|rgb| rgb[0]).collect::<Vec<u8>>()
                    }
                    BitmapBuffer::Rgba(_) => Vec::default(),
                };

                Some(GlyphImagePatch {
                    offset_x: character_cache.x,
                    offset_y: character_cache.y,
                    width: character_cache.width,
                    height: character_cache.height,
                    pixel_data: buffer.clone(),
                })
            })
            .collect();

        glyph_image_patches
    }

    pub fn invalidate_cache(&mut self) {
        self.character_data.clear();
        self.current_x = 0;
        self.current_y = 0;
    }

    pub fn get_clip_rect(&self, code: char) -> CharacterData {
        let data = match self.character_data.get(&code) {
            Some(data) => data,
            None => {
                // なければ豆腐
                self.character_data.get(&'-').unwrap()
            }
        };

        let uv_begin_x = data.x as f32 / 4096.0;
        let uv_begin_y = data.y as f32 / 4096.0;
        let uv_width = data.width as f32 / 4096.0;
        let uv_height = data.height as f32 / 4096.0;
        CharacterData {
            uv_begin: [uv_begin_x, uv_begin_y],
            uv_end: [uv_begin_x + uv_width, uv_begin_y + uv_height],
        }
    }

    // Uint_R
    // Debug 用途
    #[allow(dead_code)]
    pub fn get_buffer(&self) -> &[u8] {
        &self.buffer
    }

    #[allow(dead_code)]
    pub fn width(&self) -> u32 {
        self.image_width
    }

    #[allow(dead_code)]
    pub fn height(&self) -> u32 {
        self.image_height
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bmp::Image;
    use crossfont::RasterizedGlyph;

    use crate::detail::FontEngine;

    use super::{GlyphWriter, IGlyphManager};

    struct GlyphTable {
        table: HashMap<char, RasterizedGlyph>,
    }
    impl GlyphTable {
        pub fn new<T>(codes: T) -> Self
        where
            T: Iterator<Item = char>,
        {
            let mut font_engine = FontEngine::new_with_font_size(32.0);
            let table = codes
                .map(|code| {
                    let glyph = font_engine.rasterize(code, 32.0).unwrap();
                    (code, glyph)
                })
                .collect();

            Self { table }
        }
    }
    impl IGlyphManager for GlyphTable {
        fn acquire_rasterized_glyph(&self, code: char) -> Option<&RasterizedGlyph> {
            self.table.get(&code)
        }
    }

    // グリフ抽出の検証
    #[test]
    fn export() {
        let codes = ' '..='~';
        let glyph_table = GlyphTable::new(codes.clone());
        let mut glyph_writer = GlyphWriter::new();
        let image_patches = glyph_writer.execute(codes, &glyph_table);

        let mut image = Image::new(glyph_writer.width(), glyph_writer.height());
        for image_patch in image_patches {
            for y in 0..image_patch.height {
                for x in 0..image_patch.width {
                    if image_patch.pixels().is_empty() {
                        // 空白グリフ対策。そもそも空白はテストから除外してもいいかも。
                        continue;
                    }
                    let src_index = x + y * image_patch.width();
                    let data = image_patch.pixels()[src_index as usize];
                    let dst_x = image_patch.offset_x + x;
                    let dst_y = image_patch.offset_y + y;
                    image.set_pixel(
                        dst_x,
                        dst_y,
                        bmp::Pixel {
                            r: data,
                            g: data,
                            b: data,
                        },
                    );
                }
            }
        }

        image.save("placed_glyph.png").unwrap();
    }

    #[test]
    fn patch() {
        let codes = 'a'..'d';
        let glyph_table = GlyphTable::new(codes.clone());
        let mut glyph_writer = GlyphWriter::new();
        let image_patches = glyph_writer.execute(codes, &glyph_table);

        for image_patch in image_patches {
            let mut image = Image::new(image_patch.width, image_patch.height);
            for y in 0..image_patch.height {
                for x in 0..image_patch.width {
                    let index = (x + y * image_patch.width) as usize;

                    let pixels = image_patch.pixels();
                    let r = pixels[index];
                    let g = pixels[index];
                    let b = pixels[index];
                    image.set_pixel(x, y, bmp::Pixel { r, g, b });
                }
            }

            let file_name = format!("{}x{}.png", image_patch.offset_x, image_patch.offset_y);
            image.save(file_name).unwrap();
        }
    }
}
