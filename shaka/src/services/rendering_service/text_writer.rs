use crate::services::{
    rendering_service::{buffer_view::CharacterData, glyph_table::GlyphTable},
    shell_service::PatchData,
    utils,
};

pub trait CopyRange {
    fn new(src_offset: usize, dst_offset: usize, count: usize) -> Self;
}

pub struct TextWriter {}

impl TextWriter {
    pub fn new() -> Self {
        TextWriter {}
    }

    pub fn write<T>(
        &self,
        dst_buffer: &mut [CharacterData],
        patches: &[PatchData],
        glyph_table: &GlyphTable,
    ) -> Vec<T>
    where
        T: CopyRange,
    {
        let window_size = [1280, 960];
        let font_size = 32.0;

        let mut ranges = Vec::default();
        let count = patches.len().min(dst_buffer.len());
        for index in 0..count {
            let patch = &patches[index];
            let matrix = utils::compute_world_matrix(
                window_size,
                font_size,
                patch.content.x as u32,
                patch.content.y as u32,
            )
            .transpose();
            // 3 要素を足りない分はゼロ埋めしつつ 4 要素にする
            dst_buffer[index].transform0 = matrix.column(0).fixed_resize::<4, 1>(0.0).into();
            dst_buffer[index].transform1 = matrix.column(1).fixed_resize::<4, 1>(0.0).into();
            dst_buffer[index].fg_color = [
                patch.content.fg[0],
                patch.content.fg[1],
                patch.content.fg[2],
                1.0,
            ];

            let Some(range) = glyph_table.get_range(patch.content.code) else {
                continue;
            };
            dst_buffer[index].uv01 = range;
            ranges.push(T::new(
                index * std::mem::size_of::<CharacterData>(),
                patch.index * std::mem::size_of::<CharacterData>(),
                std::mem::size_of::<CharacterData>(),
            ));
        }

        ranges
    }
}
