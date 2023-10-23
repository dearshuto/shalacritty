use alacritty_terminal::{ansi::NamedColor, term::RenderableContent};
use nalgebra::{Matrix3, Vector2};

use super::{GlyphManager, GlyphWriter};

#[derive(PartialEq, Clone, Copy)]
pub struct CharacterInfo {
    pub code: char,
    pub transform: nalgebra::Matrix3x2<f32>,
    pub fore_ground_color: [f32; 4],
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
        let codes = vec!['─', '│', 'v', '└', '┌', '┐', '▼', '▲', '┘', '°', '…'];
        let dots = '⠀'..='⣿';
        let codes = ((1 as char)..='~')
            .chain(codes)
            .chain(dots)
            .collect::<Vec<char>>();
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
                let fore_ground_color = match cell.fg {
                    alacritty_terminal::ansi::Color::Named(c) => Self::convert_named_color(c),
                    alacritty_terminal::ansi::Color::Spec(rgb) => [
                        rgb.r as f32 / 255.0,
                        rgb.g as f32 / 255.0,
                        rgb.b as f32 / 255.0,
                        1.0f32,
                    ],
                    alacritty_terminal::ansi::Color::Indexed(i) => Self::convert_index_color(i),
                };
                CharacterInfo {
                    code,
                    transform: (matrix * scale).transpose().remove_column(2),
                    fore_ground_color,
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

    fn convert_index_color(i: u8) -> [f32; 4] {
        match i {
            0 => Self::convert_named_color(NamedColor::White),
            1 => Self::convert_named_color(NamedColor::Magenta),
            2 => Self::convert_named_color(NamedColor::Black),
            4 => Self::convert_named_color(NamedColor::White),
            6 => Self::convert_named_color(NamedColor::BrightWhite),
            7 => Self::convert_named_color(NamedColor::White),
            10 => Self::convert_named_color(NamedColor::BrightBlue),
            11 => Self::convert_named_color(NamedColor::Green),
            12 => Self::convert_named_color(NamedColor::Blue),
            13 => Self::convert_named_color(NamedColor::Cyan),
            14 => Self::convert_named_color(NamedColor::White),
            _ => {
                println!("{}", i);
                [0.0; 4]
            }
        }
    }

    fn convert_named_color(color: NamedColor) -> [f32; 4] {
        match color {
            NamedColor::Black => [0.0, 0.0, 0.0, 0.0],
            NamedColor::Red => [1.0, 0.0, 0.0, 0.0],
            NamedColor::Green => [0.0, 1.0, 0.0, 0.0],
            NamedColor::Yellow => [0.0, 1.0, 1.0, 0.0],
            NamedColor::Blue => [0.0, 0.0, 0.8, 0.0],
            NamedColor::White => [1.0, 1.0, 1.0, 0.0],
            NamedColor::Magenta => [1.0, 0.0, 1.0, 0.0],
            NamedColor::Cyan => [87.0 / 255.0, 154.0 / 255.0, 205.0 / 255.0, 0.0],
            NamedColor::BrightBlack => [0.2, 0.2, 0.2, 0.0],
            // NamedColor::BrightRed => todo!(),
            // NamedColor::BrightGreen => todo!(),
            // NamedColor::BrightYellow => todo!(),
            NamedColor::BrightBlue => [0.0, 0.0, 1.0, 0.0],
            // NamedColor::BrightMagenta => todo!(),
            // NamedColor::BrightCyan => todo!(),
            NamedColor::BrightWhite => [0.8, 0.8, 0.8, 0.0],
            NamedColor::Foreground => [1.0, 1.0, 1.0, 0.0],
            NamedColor::Background => [1.0, 1.0, 1.0, 0.0],
            // NamedColor::Cursor => todo!(),
            // NamedColor::DimBlack => todo!(),
            // NamedColor::DimRed => todo!(),
            // NamedColor::DimGreen => todo!(),
            // NamedColor::DimYellow => todo!(),
            // NamedColor::DimBlue => todo!(),
            // NamedColor::DimMagenta => todo!(),
            // NamedColor::DimCyan => todo!(),
            // NamedColor::DimWhite => todo!(),
            // NamedColor::BrightForeground => todo!(),
            // NamedColor::DimForeground => todo!(),
            _ => {
                println!("{:?}", color);
                [0.0, 0.0, 0.0, 0.0]
            }
        }
    }
}
