use alacritty_terminal::term::RenderableContent;
use nalgebra::{Matrix3, Vector2};

use super::{GlyphManager, GlyphWriter};

#[derive(PartialEq, Clone, Copy)]
pub struct CharacterInfo {
    pub code: char,
    pub transform: nalgebra::Matrix3x2<f32>,
    pub uv0: nalgebra::Vector2<f32>,
    pub uv1: nalgebra::Vector2<f32>,
}

pub struct GlyphTexturePatch {
    offset: u32,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl GlyphTexturePatch {
    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

pub struct Diff {
    glyph_texture_patches: Vec<GlyphTexturePatch>,
    character_info_array: Vec<CharacterInfo>,
}

impl Diff {
    pub fn glyph_texture_patches(&self) -> &[GlyphTexturePatch] {
        &self.glyph_texture_patches
    }

    pub fn character_info_array(&self) -> &[CharacterInfo] {
        &self.character_info_array
    }
}

pub struct ContentPlotter {
    old_items: Vec<CharacterInfo>,

    // TODO: グリフ画像を生成する処理は外部からさせるようにしたい
    glyph_writer: GlyphWriter,
}

impl ContentPlotter {
    pub fn new() -> Self {
        let glyph_writer = GlyphWriter::new();

        Self {
            old_items: Vec::default(),
            glyph_writer,
        }
    }

    pub fn calculate_diff(
        &mut self,
        _renderable_content: RenderableContent,
        glyph_manager: &GlyphManager,
    ) -> Diff {
        let glyph_patches = self
            .glyph_writer
            .execute(&['H', 'e', 'l', 'o'], glyph_manager);
        let h = self.glyph_writer.get_clip_rect('H');
        let e = self.glyph_writer.get_clip_rect('e');
        let l = self.glyph_writer.get_clip_rect('l');
        let o = self.glyph_writer.get_clip_rect('o');

        let scale: Matrix3<f32> = Matrix3::new_scaling(0.1f32);
        let matrix_0 = Matrix3::new_translation(&Vector2::new(-0.4f32, 0.0));
        let matrix_1 = Matrix3::new_translation(&Vector2::new(-0.3f32, 0.0));
        let matrix_2 = Matrix3::new_translation(&Vector2::new(-0.2f32, 0.0));
        let matrix_3 = Matrix3::new_translation(&Vector2::new(-0.1f32, 0.0));
        let matrix_4 = Matrix3::new_translation(&Vector2::new(0.0f32, 0.0));

        let items = vec![
            CharacterInfo {
                code: 'H',
                transform: (matrix_0 * scale).transpose().remove_column(2),
                uv0: nalgebra::Vector2::new(h.uv_begin[0], h.uv_begin[1]),
                uv1: nalgebra::Vector2::new(h.uv_end[0], h.uv_end[1]),
            },
            CharacterInfo {
                code: 'e',
                transform: (matrix_1 * scale).transpose().remove_column(2),
                uv0: nalgebra::Vector2::new(e.uv_begin[0], e.uv_begin[1]),
                uv1: nalgebra::Vector2::new(e.uv_end[0], e.uv_end[1]),
            },
            CharacterInfo {
                code: 'l',
                transform: (matrix_2 * scale).transpose().remove_column(2),
                uv0: nalgebra::Vector2::new(l.uv_begin[0], l.uv_begin[1]),
                uv1: nalgebra::Vector2::new(l.uv_end[0], l.uv_end[1]),
            },
            CharacterInfo {
                code: 'l',
                transform: (matrix_3 * scale).transpose().remove_column(2),
                uv0: nalgebra::Vector2::new(l.uv_begin[0], l.uv_begin[1]),
                uv1: nalgebra::Vector2::new(l.uv_end[0], l.uv_end[1]),
            },
            CharacterInfo {
                code: 'o',
                transform: (matrix_4 * scale).transpose().remove_column(2),
                uv0: nalgebra::Vector2::new(o.uv_begin[0], o.uv_begin[1]),
                uv1: nalgebra::Vector2::new(o.uv_end[0], o.uv_end[1]),
            },
        ];

        if self.old_items == items {
            return Diff {
                glyph_texture_patches: Vec::default(),
                character_info_array: Vec::default(),
            };
        }

        self.old_items = items.clone();

        // グリフ
        let glyph_patch = &glyph_patches[0];
        let texture_patch = GlyphTexturePatch {
            offset: 0,
            width: glyph_patch.width(),
            height: glyph_patch.height(),
            pixels: glyph_patch.pixels().to_vec(),
        };

        return Diff {
            glyph_texture_patches: vec![texture_patch],
            character_info_array: items,
        };
    }
}
