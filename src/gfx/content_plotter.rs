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
        renderable_content: RenderableContent,
        glyph_manager: &GlyphManager,
    ) -> Diff {
        // 使いそうなグリフが詰まった画像を用意
        let codes = ((0 as char)..='~').collect::<Vec<char>>();
        let glyph_patches = self.glyph_writer.execute(&&codes, glyph_manager);

        let items = renderable_content
            .display_iter
            .map(|cell| {
                // [-1, 1] の範囲
                // ウィンドウバーの分だけちょっとずらしてる
                let position_normalized = 2.0
                    * Vector2::new(cell.point.column.0 as f32, cell.point.line.0 as f32)
                    / 64.0f32
                    - Vector2::new(0.98, 0.98);
                let code = cell.c;
                let glyph = glyph_manager.get_rasterized_glyph(code);
                // GlyphManager がフォントサイズ 32 決め打ちでラスタライズしている
                let scale: Matrix3<f32> = Matrix3::new_nonuniform_scaling(&Vector2::new(
                    0.035 * (glyph.width as f32 / 32.0),
                    0.035 * (glyph.height as f32 / 32.0),
                ));
                let matrix = Matrix3::new_translation(&position_normalized);
                let character = self.glyph_writer.get_clip_rect(code);
                CharacterInfo {
                    code,
                    transform: (matrix * scale).transpose().remove_column(2),
                    uv0: nalgebra::Vector2::new(character.uv_begin[0], character.uv_begin[1]),
                    uv1: nalgebra::Vector2::new(character.uv_end[0], character.uv_end[1]),
                }
            })
            .collect::<Vec<CharacterInfo>>();

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
