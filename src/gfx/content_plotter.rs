use alacritty_terminal::{
    index::{Column, Line, Point},
    vte::ansi::Color,
};

use crate::util::DiffCalculator;

use super::{
    detail::{BufferPatch, GlyphImagePatch},
    GlyphManager,
};

pub trait IContent {
    type TColor;
    type TPosition;

    fn code(&self) -> char;

    fn color_fg(&self) -> Self::TColor;

    fn position(&self) -> Self::TPosition;
}

#[derive(PartialEq, Clone, Copy)]
pub struct CharacterInfo {
    pub code: char,
    pub transform: nalgebra::Matrix3x2<f32>,
    pub fore_ground_color: [f32; 4],
    pub uv0: nalgebra::Vector2<f32>,
    pub uv1: nalgebra::Vector2<f32>,
    pub index: usize,
}

#[derive(Debug)]
pub struct GlyphTexturePatch {
    offset_x: u32,
    offset_y: u32,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl GlyphTexturePatch {
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
        &self.pixels
    }
}

impl From<GlyphImagePatch> for GlyphTexturePatch {
    fn from(value: GlyphImagePatch) -> Self {
        Self {
            offset_x: value.offset_x(),
            offset_y: value.offset_y(),
            width: value.width(),
            height: value.height(),
            pixels: value.pixels().to_vec(),
        }
    }
}

#[derive(Default)]
pub struct Diff {
    buffer_patches: Vec<BufferPatch>,
    cursor: Option<Point>,
    item_count: i32,
}

impl Diff {
    pub fn buffer_patches(&self) -> &[BufferPatch] {
        &self.buffer_patches
    }

    pub fn cursor(&self) -> Option<&Point> {
        self.cursor.as_ref()
    }

    pub fn item_count(&self) -> i32 {
        self.item_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterInfoCache {
    pub code: char,
    pub color: Color,
    pub point: Point<Line, Column>,
}

pub struct ContentPlotter {
    // 差分検出
    diff_calculator: DiffCalculator<CharacterInfoCache>,
}

impl ContentPlotter {
    pub fn new() -> Self {
        let diff_calculator = DiffCalculator::new();

        Self { diff_calculator }
    }

    pub fn calculate_diff<TCells, TContent>(
        &mut self,
        cells: TCells,
        cursor_point: &Point,
        glyph_manager: &GlyphManager,
        size: (u32, u32),
    ) -> Diff
    where
        TCells: Iterator<Item = TContent>,
        TContent: IContent<TColor = Color, TPosition = Point>,
    {
        // 差分検出
        let items = cells
            .map(|c| CharacterInfoCache {
                code: c.code(),
                color: c.color_fg(),
                point: c.position(),
            })
            .collect::<Vec<CharacterInfoCache>>();
        let item_count = items.len();

        let buffer_patches =
            self.diff_calculator
                .calculate_binary_patch(&items, glyph_manager, size);

        Diff {
            buffer_patches,
            cursor: Some(*cursor_point),
            item_count: item_count as i32,
        }
    }
}
