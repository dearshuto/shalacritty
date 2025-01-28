pub trait IBuffer {
    type Data;

    fn write(&mut self, index: usize, data: &Self::Data);
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

    pub fn plot<TBuffer, TItemIterator, TContent>(
        &self,
        out_buffer: &mut TBuffer,
        items: TItemIterator,
    ) where
        TBuffer: IBuffer<Data = i32>,
        TItemIterator: Iterator<Item = TContent>,
        TContent: IContent,
    {
        let mut base_point = (0.0f32, 0.0f32);
        for (index, item) in items.enumerate() {
            // 改行がきたら次の行に送る
            if item.code() == '\n' {
                base_point = (0.0f32, base_point.1 + self.font_size + self.line_spacing);
                continue;
            }

            // ピクセル座標で 1x1 の四角形をフォントのサイズにスケール
            // let local_pixel_scale_matrix = Matrix3::new_nonuniform_scaling(&Vector2::new(
            //     glyph.width as f32,
            //     glyph.height as f32,
            // ));
            // out_buffer.write(index, &12);

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
