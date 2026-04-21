use crate::services::{
    rendering_service::{buffer_view::CharacterData, glyph_table::GlyphTable},
    shell_service::Patch,
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
        patches: &[Patch],
        glyph_table: &GlyphTable,
    ) -> usize {
        let window_size = [1280, 960];
        let font_size = 32.0;

        let count = patches.len().min(dst_buffer.len());
        for index in 0..count {
            let content = &patches[index];
            let matrix = utils::compute_world_matrix(
                window_size,
                font_size,
                content.x as u32,
                content.y as u32,
            )
            .transpose();
            // 3 要素を足りない分はゼロ埋めしつつ 4 要素にする
            dst_buffer[index].transform0 = matrix.column(0).fixed_resize::<4, 1>(0.0).into();
            dst_buffer[index].transform1 = matrix.column(1).fixed_resize::<4, 1>(0.0).into();
            dst_buffer[index].fg_color = [0.0, 0.8, 0.0, 1.0];

            let Some(range) = glyph_table.get_range(content.code) else {
                continue;
            };
            dst_buffer[index].uv01 = range;
        }

        count
    }
}
