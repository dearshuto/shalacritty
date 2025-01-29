use nalgebra::{Matrix3, Matrix3x4, Vector2};

pub trait IBuffer<T> {
    fn write(&mut self, index: usize, data: &T);
}

pub trait IConverter {
    type Data;

    fn convert(&self, data: &CharacterData) -> Self::Data;
}

pub trait IContent {
    type TColor;
    type TPosition;

    fn code(&self) -> char;

    fn color_fg(&self) -> Self::TColor;

    fn width(&self) -> u32;

    fn height(&self) -> u32;

    fn bottom(&self) -> i32;

    fn left(&self) -> i32;

    // 横書きを想定
    fn advance(&self) -> f32;
}

pub struct CharacterData {
    pub code: char,
    pub transform: Matrix3<f32>,
}

pub struct ContentPlotter {
    font_size: f32,
    line_spacing: f32,
    window_width: u32,
    window_height: u32,
}

impl ContentPlotter {
    pub fn new() -> Self {
        Self {
            font_size: 24.0,
            line_spacing: 1.0,
            window_width: 640,
            window_height: 480,
        }
    }

    pub fn plot<TBuffer, TItemIterator, TContent, TConverter>(
        &self,
        out_buffer: &mut TBuffer,
        items: TItemIterator,
        converter: &TConverter,
    ) where
        TBuffer: IBuffer<TConverter::Data>,
        TItemIterator: Iterator<Item = TContent>,
        TContent: IContent,
        TConverter: IConverter,
    {
        let mut base_point = (0.0f32, 0.0f32);
        for (index, item) in items.enumerate() {
            // 改行がきたら次の行に送る
            if item.code() == '\n' {
                base_point = (0.0f32, base_point.1 + self.font_size + self.line_spacing);
                continue;
            }

            // ピクセル座標で 1x1 の四角形をフォントのサイズにスケール
            let local_pixel_scale_matrix = Matrix3::new_nonuniform_scaling(&Vector2::new(
                item.width() as f32,
                item.height() as f32,
            ));

            // ピクセル座標で表示位置をずらす
            let local_pixel_translate_matrix =
                Matrix3::new_translation(&Vector2::new(item.left() as f32, item.bottom() as f32));

            // ピクセル座標を [0, 1] 空間に変換する行列
            // フレームバッファーのサイズで変わる
            // 文字間を開けて見栄えを整えるために文字サイズを 0.6 倍している
            let normalized_matrix = Matrix3::new_nonuniform_scaling(&Vector2::new(
                0.6f32 / self.window_width as f32,
                0.6f32 / self.window_height as f32,
            ));

            // [0, 1] => [-1, 1]
            let view_matrix =
                Matrix3::new_translation(&Vector2::new(-1.0, -1.0)) * Matrix3::new_scaling(2.0);

            // 画面上に配置
            // let offset_matrix = Matrix3::new_translation(
            //     &(Vector2::new(
            //          as f32 / (size.0 as f32 / 16.0),
            //         item.point.line() as f32 / (size.1 as f32 / 16.0),
            //     )),
            // );
            let offset_matrix = Matrix3::identity();

            let transform = view_matrix
                * offset_matrix
                * normalized_matrix
                * local_pixel_translate_matrix
                * local_pixel_scale_matrix;

            let data = CharacterData {
                code: item.code(),
                transform,
            };

            let write_data = converter.convert(&data);
            out_buffer.write(index, &write_data);

            // 次の文字の開始点まで送る
            base_point.0 += item.advance();
        }
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }

    pub fn set_line_spacing(&mut self, value: f32) {
        self.line_spacing = value;
    }
}
