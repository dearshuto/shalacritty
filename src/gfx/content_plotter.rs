use alacritty_terminal::term::RenderableContent;
use nalgebra::{Matrix3, Vector2};

use super::GlyphManager;

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
}

impl ContentPlotter {
    pub fn new() -> Self {
        Self {
            old_items: Vec::default(),
        }
    }

    pub fn calculate_diff(
        &mut self,
        _renderable_content: RenderableContent,
        glyph_manager: &GlyphManager,
    ) -> Diff {
        let scale: Matrix3<f32> = Matrix3::new_scaling(0.5f32);
        let matrix_0 = Matrix3::new_translation(&Vector2::new(-0.3f32, 0.0));
        let matrix_1 = Matrix3::new_translation(&Vector2::new(0.3f32, 0.0));

        let items = vec![
            CharacterInfo {
                code: 'H',
                transform: (matrix_0 * scale).transpose().remove_column(2),
                uv0: nalgebra::Vector2::default(),
                uv1: nalgebra::Vector2::default(),
            },
            CharacterInfo {
                code: 'e',
                transform: (matrix_1 * scale).transpose().remove_column(2),
                uv0: nalgebra::Vector2::default(),
                uv1: nalgebra::Vector2::default(),
            }, /*
               CharacterInfo {
                   code: 'l',
                   transform: nalgebra::Matrix3x2::identity(),
                   uv0: nalgebra::Vector2::default(),
                   uv1: nalgebra::Vector2::default(),
               },
               CharacterInfo {
                   code: 'l',
                   transform: nalgebra::Matrix3x2::identity(),
                   uv0: nalgebra::Vector2::default(),
                   uv1: nalgebra::Vector2::default(),
               },
               CharacterInfo {
                   code: 'o',
                   transform: nalgebra::Matrix3x2::identity(),
                   uv0: nalgebra::Vector2::default(),
                   uv1: nalgebra::Vector2::default(),
               },*/
        ];

        if self.old_items == items {
            return Diff {
                glyph_texture_patches: Vec::default(),
                character_info_array: Vec::default(),
            };
        }

        self.old_items = items.clone();

        // とりあえず何かしらのグリフを GPU に送る
        let glyph = glyph_manager.get_rasterized_glyph('F');
        let mut data = Vec::default();
        let glyph_data = {
            match &glyph.buffer {
                crossfont::BitmapBuffer::Rgb(buffer) => buffer,
                crossfont::BitmapBuffer::Rgba(buffer) => buffer,
            }
        };
        for index in 0..(glyph.width * glyph.height) {
            let r = glyph_data[3 * index as usize];
            data.push(r);
        }

        let texture_patch = GlyphTexturePatch {
            offset: 0,
            width: glyph.width as u32,
            height: glyph.height as u32,
            pixels: data,
        };

        return Diff {
            glyph_texture_patches: vec![texture_patch],
            character_info_array: items,
        };
    }
}
