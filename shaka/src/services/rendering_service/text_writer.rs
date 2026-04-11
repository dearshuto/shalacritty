use crate::services::{
    rendering_service::{buffer_view::CharacterData, glyph_table::GlyphTable},
    utils,
};

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
        let window_size = [1280, 960];
        let font_size = 32.0;

        let mut count = 0;
        let mut current_x = 0;
        let mut current_y = 0;
        for index in 0..str.len().min(dst_buffer.len()) {
            if str.chars().nth(index).unwrap() == '\n' {
                current_x = 0;
                current_y += 1;
                continue;
            }
            let matrix = utils::compute_world_matrix(window_size, font_size, current_x, current_y)
                .transpose();
            // 3 要素を足りない分はゼロ埋めしつつ 4 要素にする
            dst_buffer[index].transform0 = matrix.column(0).fixed_resize::<4, 1>(0.0).into();
            dst_buffer[index].transform1 = matrix.column(1).fixed_resize::<4, 1>(0.0).into();
            dst_buffer[index].fg_color = [0.0, 0.8, 0.0, 1.0];
            current_x += 1;

            let Some(range) = glyph_table.get_range(str.chars().nth(index).unwrap()) else {
                continue;
            };
            dst_buffer[index].uv01 = range;
            count += 1;
        }

        count
    }
}
