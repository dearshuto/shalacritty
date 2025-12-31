use crate::services::rendering_service::{buffer_view::CharacterData, glyph_table::GlyphTable};

pub struct TextWriter {}

impl TextWriter {
    pub fn new() -> Self {
        TextWriter {}
    }

    pub fn write(
        &self,
        dst_buffer: &mut [CharacterData],
        str: &str,
        glyph_table: &GlyphTable,
    ) -> usize {
        let mut count = 0;
        for index in 0..str.len() {
            let x = -0.9 + index as f32 * 0.3;
            dst_buffer[index].transform0 = [0.1, 0.0, x, 0.0];
            dst_buffer[index].transform1 = [0.0, 0.2, -0.5, 0.0];
            dst_buffer[index].fg_color = [0.0, 0.8, 0.0, 1.0];

            let Some(range) = glyph_table.get_range(str.chars().nth(index).unwrap()) else {
                continue;
            };
            dst_buffer[index].uv01 = range;
            count += 1;
        }

        count * std::mem::size_of::<CharacterData>()
    }
}
