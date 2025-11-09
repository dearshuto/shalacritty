use crossfont::{FontDesc, Rasterize, Slant, Style, Weight};

#[derive(Debug)]
pub struct GlyphRequest {
    string: [char; 64],
    response: tokio::sync::oneshot::Sender<GlyphResponse>,
}

impl GlyphRequest {
    pub fn new(
        str: &[char],
        response: tokio::sync::oneshot::Sender<GlyphResponse>,
    ) -> Result<Self, ()> {
        if str.len() >= 64 {
            return Err(());
        }

        let mut me = Self {
            string: [char::default(); 64],
            response,
        };
        me.string[0..str.len()].copy_from_slice(str);

        Ok(me)
    }
}

#[derive(Debug)]
pub struct GlyphResponse {
    glyphs: [u8; std::mem::size_of::<crossfont::RasterizedGlyph>() * 64],
    size: usize,
}

impl GlyphResponse {
    pub fn glyphs(&self) -> &[crossfont::RasterizedGlyph] {
        unsafe {
            std::slice::from_raw_parts(
                self.glyphs.as_ptr() as *const crossfont::RasterizedGlyph,
                self.size,
            )
        }
    }
}

#[derive(Clone)]
struct Config {
    font_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self { font_size: 11.0 }
    }
}

pub struct GlyphExtractService {
    request_receiver: tokio::sync::mpsc::Receiver<GlyphRequest>,

    rasterizer: crossfont::Rasterizer,

    font_key: Option<crossfont::FontKey>,
    current_font_size: Option<f32>,

    // 遅延ラスタライズ
    lazy_string: Option<String>,
}

impl GlyphExtractService {
    pub fn new(request_receiver: tokio::sync::mpsc::Receiver<GlyphRequest>) -> Self {
        Self {
            request_receiver,
            rasterizer: crossfont::Rasterizer::new().unwrap(),
            font_key: None,
            current_font_size: None,
            lazy_string: None,
        }
    }

    async fn serve(mut self, mut token: renge::CancellationToken) {
        // TODO: 設定のリアルタイム更新対応
        self.apply_config(Config::default()).await;

        loop {
            tokio::select!(
            Some(request) = self.request_receiver.recv() => self.handle_request(request).await,
            _ = &mut token => break,
            else => break,
            )
        }
    }

    async fn handle_request(&mut self, request: GlyphRequest) {
        let str = &request.string;

        // フォントサイズが不明な状態だとなにもしない
        let Some(font_size) = self.current_font_size else {
            self.lazy_string = Some(str.iter().collect());
            return;
        };

        // FontKey がなければグリフが決まらないのでなにもしない
        let Some(font_key) = self.font_key else {
            self.lazy_string = Some(str.iter().collect());
            return;
        };

        let mut glyph_response = GlyphResponse {
            glyphs: [0; _],
            size: 0,
        };

        // ラスタライズ
        let mut rasterized_count = 0;
        for c in str {
            // ラスタライズ済みなら何もしない
            let Ok(glyph) = self.rasterizer.get_glyph(crossfont::GlyphKey {
                character: *c,
                font_key,
                size: crossfont::Size::new(font_size),
            }) else {
                // TODO: 豆腐対応
                continue;
            };

            let ptr = unsafe {
                glyph_response.glyphs.as_ptr().byte_offset(
                    (rasterized_count * std::mem::size_of::<crossfont::RasterizedGlyph>()) as isize,
                )
            } as *mut crossfont::RasterizedGlyph;
            unsafe { *ptr = glyph };
            rasterized_count += 1;
        }

        glyph_response.size = rasterized_count;

        request.response.send(glyph_response).unwrap();
    }

    async fn apply_config(&mut self, config: Config) {
        // そもそも FontKey がなければ作る
        if self.font_key.is_none() {
            let font_key = Self::create_font_key(&mut self.rasterizer, config.font_size);
            self.font_key = Some(font_key);
        }

        // フォントサイズに変更がなければなにもしない
        if let Some(current_font_size) = self.current_font_size {
            if current_font_size == config.font_size {
                return;
            }
        }

        self.current_font_size = Some(config.font_size);

        // // 遅延初期化分
        // if let Some(lazy_str) = self.lazy_string.take() {
        //     self.extract(&lazy_str.chars().collect()).await;
        // }
    }

    fn create_font_key(
        rasterizer: &mut crossfont::Rasterizer,
        font_size: f32,
    ) -> crossfont::FontKey {
        rasterizer
            .load_font(
                &FontDesc::new(
                    #[cfg(not(any(target_os = "macos", windows)))]
                    "monospace",
                    #[cfg(target_os = "macos")]
                    "Menlo",
                    #[cfg(target_os = "windows")]
                    "Consolas",
                    Style::Description {
                        slant: Slant::Normal,
                        weight: Weight::Normal,
                    },
                ),
                crossfont::Size::new(font_size),
            )
            .unwrap()
    }
}

impl std::fmt::Debug for GlyphExtractService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GlyphExtractService")?;
        std::fmt::Result::Ok(())
    }
}

impl renge::Service for GlyphExtractService {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
