use alacritty_terminal::{
    grid::Indexed,
    term::{cell::Cell, RenderableContent, RenderableCursor},
    vte::ansi::{Color, NamedColor},
};

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

pub struct Diff {
    glyph_texture_patches: Vec<GlyphTexturePatch>,
    character_info_array: Vec<CharacterInfo>,
    cursor: Option<RenderableCursor>,
}

impl Diff {
    pub fn glyph_texture_patches(&self) -> &[GlyphTexturePatch] {
        &self.glyph_texture_patches
    }

    pub fn character_info_array(&self) -> &[CharacterInfo] {
        &self.character_info_array
    }

    pub fn cursor(&self) -> Option<&RenderableCursor> {
        self.cursor.as_ref()
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
        glyph_manager: &mut GlyphManager,
    ) -> Diff {
        // グリフは全部作り直してる。差分検出したい
        let cells = renderable_content
            .display_iter
            .collect::<Vec<Indexed<&Cell>>>();
        let codes = cells.iter().map(|c| c.c).collect::<Vec<char>>();

        let glyph_patches = self.glyph_writer.execute(&codes, glyph_manager);

        // 表示要素を描画に必要な情報に変換
        let items = cells
            .iter()
            .map(|cell| {
                let code = cell.c;
                let glyph = glyph_manager.get_rasterized_glyph(code);

                // ピクセル座標で 1x1 の四角形をフォントのサイズにスケール
                let local_pixel_scale_matrix = Matrix3::new_nonuniform_scaling(&Vector2::new(
                    glyph.width as f32,
                    glyph.height as f32,
                ));

                // ピクセル座標で表示位置をずらす
                let local_pixel_translate_matrix = Matrix3::new_translation(&Vector2::new(
                    glyph.left as f32,
                    (32 - glyph.top) as f32,
                ));

                // ピクセル座標を [0, 1] 空間に変換する行列
                // フレームバッファーのサイズで変わる
                // 文字間を開けて見栄えを整えるために文字サイズを 0.6 倍している
                let normalized_matrix = Matrix3::new_nonuniform_scaling(&Vector2::new(
                    0.6f32 / 1600.0,
                    0.6f32 / 1200.0,
                ));

                // [0, 1] => [-1, 1]
                let view_matrix =
                    Matrix3::new_translation(&Vector2::new(-1.0, -1.0)) * Matrix3::new_scaling(2.0);

                // 画面上に配置
                let offset_matrix = Matrix3::new_translation(
                    &(Vector2::new(
                        cell.point.column.0 as f32 / 32.0,
                        cell.point.line.0 as f32 / 32.0,
                    )),
                );

                let transform_matrix = offset_matrix
                    * view_matrix
                    * normalized_matrix
                    * local_pixel_translate_matrix
                    * local_pixel_scale_matrix;

                let character = self.glyph_writer.get_clip_rect(code);
                let fore_ground_color = match cell.fg {
                    Color::Named(c) => Self::convert_named_color(c),
                    Color::Spec(rgb) => [
                        rgb.r as f32 / 255.0,
                        rgb.g as f32 / 255.0,
                        rgb.b as f32 / 255.0,
                        1.0f32,
                    ],
                    Color::Indexed(i) => Self::convert_index_color(i),
                };
                CharacterInfo {
                    code,
                    transform: transform_matrix.transpose().remove_column(2),
                    fore_ground_color,
                    uv0: nalgebra::Vector2::new(character.uv_begin[0], character.uv_begin[1]),
                    uv1: nalgebra::Vector2::new(character.uv_end[0], character.uv_end[1]),
                }
            })
            .collect::<Vec<CharacterInfo>>();

        // 差分検出
        let mut diff_items = Vec::default();
        (0..items.len()).for_each(|index| {
            let new_item = items[index];

            // 古い要素がなかったら新規要素として追加
            let Some(old_item) = self.old_items.get(index) else {
                diff_items.push(new_item);
                return;
            };

            // 差分がなければ何もしない
            if old_item == &new_item {
                return;
            }

            diff_items.push(new_item);
        });

        // 差分がなければ更新する要素はない
        if diff_items.is_empty() {
            return Diff {
                glyph_texture_patches: Vec::default(),
                character_info_array: Vec::default(),
                cursor: Some(renderable_content.cursor),
            };
        }

        // 新たな値をキャッシュ。次の差分検出に使う。
        self.old_items = items.clone();

        // グリフ
        let glyph_texture_patches = glyph_patches
            .iter()
            .map(|glyph_patch| {
                //
                GlyphTexturePatch {
                    offset_x: glyph_patch.offset_x(),
                    offset_y: glyph_patch.offset_y(),
                    width: glyph_patch.width(),
                    height: glyph_patch.height(),
                    pixels: glyph_patch.pixels().to_vec(),
                }
            })
            .collect::<Vec<GlyphTexturePatch>>();

        Diff {
            glyph_texture_patches,
            character_info_array: items,
            cursor: Some(renderable_content.cursor),
        }
    }

    fn convert_index_color(i: u8) -> [f32; 4] {
        match i {
            0 => Self::convert_named_color(NamedColor::White),
            1 => Self::convert_named_color(NamedColor::Magenta),
            2 => Self::convert_named_color(NamedColor::Black),
            3 => Self::convert_named_color(NamedColor::BrightBlack),
            4 => Self::convert_named_color(NamedColor::White),
            5 => Self::convert_named_color(NamedColor::BrightMagenta),
            6 => Self::convert_named_color(NamedColor::BrightWhite),
            7 => Self::convert_named_color(NamedColor::White),
            8 => Self::convert_named_color(NamedColor::BrightBlack),
            10 => Self::convert_named_color(NamedColor::BrightBlue),
            11 => Self::convert_named_color(NamedColor::Green),
            12 => Self::convert_named_color(NamedColor::Blue),
            13 => Self::convert_named_color(NamedColor::Cyan),
            14 => Self::convert_named_color(NamedColor::White),
            15 => Self::convert_named_color(NamedColor::Green),
            _ => {
                println!("unknown index color: {}", i);
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
            NamedColor::BrightMagenta => [1.0, 0.0, 1.0, 0.0],
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
                println!("unknown color: {:?}", color);
                [0.0, 0.0, 0.0, 0.0]
            }
        }
    }
}
