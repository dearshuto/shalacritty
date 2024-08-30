use alacritty_terminal::vte::ansi::{Color, NamedColor};
use nalgebra::{Matrix3, Vector2};

use crate::gfx::{BufferPatch, CharacterDataPatch, CharacterInfoCache, GlyphManager};

pub struct Diff<T> {
    items: Vec<T>,
    indeicies: Vec<usize>,
}

impl<T> Diff<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn indicies(&self) -> &[usize] {
        &self.indeicies
    }
}

pub trait IDiffCalculator<T> {
    fn calculate(&mut self, items: &[T]) -> Diff<T>;
}

pub struct DiffCalculator<T>
where
    T: Eq + Copy,
{
    old_items: Vec<T>,
}

impl<T> DiffCalculator<T>
where
    T: Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            old_items: Vec::default(),
        }
    }
}

impl DiffCalculator<CharacterInfoCache> {
    pub fn calculate_binary_patch(
        &mut self,
        items: &[CharacterInfoCache],
        glyph_manager: &GlyphManager,
        size: (u32, u32),
    ) -> Vec<BufferPatch> {
        // 要素を列挙して比較するシンプルな実装
        // Wu の差分検出みたいないけてる実装に載せ替えたい

        let old_items: Vec<_> = items.to_vec();

        let mut buffer_patches = Vec::default();
        for (index, item) in old_items.iter().enumerate() {
            let Some(old_item) = self.old_items.get(index) else {
                let buffer_patch = Self::create_character_data_patch(
                    index,
                    &CharacterInfoCache {
                        code: ' ',
                        color: Color::Indexed(0),
                        point: Default::default(),
                    },
                    &items[index],
                    glyph_manager,
                    size,
                );
                buffer_patches.push(buffer_patch.into());
                continue;
            };

            if old_item == item {
                continue;
            }

            let buffer_patch = Self::create_character_data_patch(
                index,
                old_item,
                &items[index],
                glyph_manager,
                size,
            );
            buffer_patches.push(buffer_patch.into());
        }

        self.old_items = old_items;

        buffer_patches
    }

    fn create_character_data_patch(
        index: usize,
        old_value: &CharacterInfoCache,
        new_value: &CharacterInfoCache,
        glyph_manager: &GlyphManager,
        size: (u32, u32),
    ) -> CharacterDataPatch {
        let (transform0, transform1) = if old_value.code == new_value.code
            && old_value.point == new_value.point
        {
            (None, None)
        } else {
            let code = new_value.code;
            let glyph = glyph_manager.get_rasterized_glyph(code);

            // ピクセル座標で 1x1 の四角形をフォントのサイズにスケール
            let local_pixel_scale_matrix = Matrix3::new_nonuniform_scaling(&Vector2::new(
                glyph.width as f32,
                glyph.height as f32,
            ));

            // ピクセル座標で表示位置をずらす
            let local_pixel_translate_matrix =
                Matrix3::new_translation(&Vector2::new(glyph.left as f32, (32 - glyph.top) as f32));

            // ピクセル座標を [0, 1] 空間に変換する行列
            // フレームバッファーのサイズで変わる
            // 文字間を開けて見栄えを整えるために文字サイズを 0.6 倍している
            let normalized_matrix = Matrix3::new_nonuniform_scaling(&Vector2::new(
                0.6f32 / size.0 as f32,
                0.6f32 / size.1 as f32,
            ));

            // [0, 1] => [-1, 1]
            let view_matrix =
                Matrix3::new_translation(&Vector2::new(-1.0, -1.0)) * Matrix3::new_scaling(2.0);

            // 画面上に配置
            let offset_matrix = Matrix3::new_translation(
                &(Vector2::new(
                    new_value.point.column.0 as f32 / (size.0 as f32 / 16.0),
                    new_value.point.line.0 as f32 / (size.1 as f32 / 16.0),
                )),
            );

            let transform_matrix = (view_matrix
                * offset_matrix
                * normalized_matrix
                * local_pixel_translate_matrix
                * local_pixel_scale_matrix)
                .transpose()
                .remove_column(2);
            (
                Some([transform_matrix.m11, transform_matrix.m21, 0.0f32, 0.0f32]),
                Some([transform_matrix.m12, transform_matrix.m22, 0.0f32, 0.0f32]),
            )
        };

        // カラー
        let foreground_color = if old_value.color == new_value.color {
            None
        } else {
            let foreground_color = match new_value.color {
                Color::Named(c) => Self::convert_named_color(c),
                Color::Spec(rgb) => [
                    rgb.r as f32 / 255.0,
                    rgb.g as f32 / 255.0,
                    rgb.b as f32 / 255.0,
                    1.0f32,
                ],
                Color::Indexed(i) => Self::convert_index_color(i),
            };
            Some(foreground_color)
        };

        // グリフの UV
        let (uv_bl, uv_tr) = if old_value.code == new_value.code {
            (None, None)
        } else {
            let character = glyph_manager.get_clip_rect(new_value.code);
            let uv_bl = nalgebra::Vector2::new(character.uv_begin[0], character.uv_begin[1]);
            let uv_tr = nalgebra::Vector2::new(character.uv_end[0], character.uv_end[1]);
            (Some([uv_bl.x, uv_bl.y]), Some([uv_tr.x, uv_tr.y]))
        };

        CharacterDataPatch {
            index,
            transform0,
            transform1,
            foreground_color,
            uv_bl,
            uv_tr,
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
            // わからん
            48 => Self::convert_named_color(NamedColor::White),
            81 => Self::convert_named_color(NamedColor::White),
            149 => Self::convert_named_color(NamedColor::White),
            208 => Self::convert_named_color(NamedColor::White),
            243 => Self::convert_named_color(NamedColor::White),
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
            NamedColor::Yellow => [1.0, 1.0, 0.0, 0.0],
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
            NamedColor::BrightCyan => [0.0, 1.0, 1.0, 0.0],
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

impl<T> IDiffCalculator<T> for DiffCalculator<T>
where
    T: Eq + Copy,
{
    fn calculate(&mut self, items: &[T]) -> Diff<T> {
        // 要素を列挙して比較するシンプルな実装
        // Wu の差分検出みたいないけてる実装に載せ替えたい

        let old_items: Vec<T> = items.to_vec();

        let mut changed_items = Vec::default();
        let mut item_indicies = Vec::default();
        for (index, item) in old_items.iter().enumerate() {
            let Some(old_item) = self.old_items.get(index) else {
                changed_items.push(*item);
                item_indicies.push(index);
                continue;
            };

            if old_item == item {
                continue;
            }

            changed_items.push(*item);
            item_indicies.push(index);
        }

        self.old_items = old_items;

        Diff {
            items: changed_items,
            indeicies: item_indicies,
        }
    }
}
