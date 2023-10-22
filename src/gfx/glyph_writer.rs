use std::collections::HashMap;

use crossfont::BitmapBuffer;

use super::GlyphManager;

pub struct CharacterData {
    pub uv_begin: [f32; 2],
    pub uv_end: [f32; 2],
}

pub struct GlyphImagePatch {
    #[allow(dead_code)]
    offset_x: u32,
    #[allow(dead_code)]
    offset_y: u32,
    width: u32,
    height: u32,
    pixel_data: Vec<u8>,
}

impl GlyphImagePatch {
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

pub struct GlyphWriter {
    image_width: u32,
    image_height: u32,

    // キャッシュ
    character_data: HashMap<char, CharacterData>,
}

impl GlyphWriter {
    pub fn new() -> Self {
        Self {
            image_width: 32 * 32, /*32 ピクセルを 32 文字で 1024 ピクセル*/
            image_height: 32 * 32,
            character_data: HashMap::default(),
        }
    }

    pub fn execute(
        &mut self,
        codes: &[char],
        glyph_manager: &GlyphManager,
    ) -> Vec<GlyphImagePatch> {
        // とりあえず毎回作り直す
        self.character_data.clear();

        let mut result = Vec::default();
        result.resize((self.image_width * self.image_height) as usize, 0);

        let mut current_count_x = 0;
        let mut current_count_y = 0;
        for code in codes {
            let glyph = glyph_manager.get_rasterized_glyph(*code);
            let offset_x = current_count_x * 32;
            let offset_y = current_count_y * 32;

            let BitmapBuffer::Rgb(buffer) = &glyph.buffer else {
                continue;
            };
            for y in 0..glyph.height as u32 {
                for x in 0..glyph.width as u32 {
                    let data = buffer[3 * (x + y * glyph.width as u32) as usize];
                    let dst_index = (offset_x + x) + (y + offset_y) * self.image_height;
                    result[dst_index as usize] = data;
                }
            }

            // 画像のどこに文字が配置されたかの情報
            let uv_width = 32.0f32 / 1024.0;
            let uv_height = 32.0f32 / 1024.0;
            let uv_begin_x = current_count_x as f32 * uv_width;
            let uv_begin_y = current_count_y as f32 * uv_height;
            let uv_width = glyph.width as f32 / 1024.0;
            let uv_height = glyph.height as f32 / 1024.0;
            let character_data = CharacterData {
                uv_begin: [uv_begin_x, uv_begin_y],
                uv_end: [uv_begin_x + uv_width, uv_begin_y + uv_height],
            };
            self.character_data.insert(*code, character_data);

            // x がはじまで到達したら、y は次の行に移動して x は先頭に戻る
            current_count_y += (current_count_x + 1) / 32;
            current_count_x = current_count_x + 1;
        }

        // ひとまず全部作り直してるので全領域をパッチとして返す
        vec![GlyphImagePatch {
            offset_x: 0,
            offset_y: 0,
            width: self.image_width,
            height: self.image_height,
            pixel_data: result,
        }]
    }

    pub fn get_clip_rect(&self, code: char) -> &CharacterData {
        self.character_data.get(&code).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use bmp::Image;

    use crate::gfx::GlyphManager;

    use super::GlyphWriter;

    // グリフ抽出の検証
    #[test]
    fn export() {
        let mut glyph_manager = GlyphManager::new();
        glyph_manager.extract_alphabet();

        let mut glyph_writer = GlyphWriter::new();
        let codes = vec!['a', 'b', 'c'];
        let image_patches = glyph_writer.execute(&codes, &glyph_manager);
        let image_patch = &image_patches[0];

        let mut image = Image::new(image_patch.width, image_patch.height);
        for y in image_patch.offset_y..image_patch.height {
            for x in image_patch.offset_x..image_patch.width {
                let index = (x + y * image_patch.width) as usize;
                let pixels = image_patch.pixels();
                let r = pixels[index];
                let g = pixels[index];
                let b = pixels[index];
                image.set_pixel(x as u32, y as u32, bmp::Pixel { r, g, b });
            }
        }

        image.save("placed_glyph.png").unwrap();
    }
}
