use std::collections::{HashMap, HashSet};

use crossfont::{FontDesc, Rasterize, RasterizedGlyph, Slant, Style, Weight};

use crate::config_service::Config;

pub struct ExtractionInfo {
    pub glyphs: HashMap<char, RasterizedGlyph>,
}

pub struct GlyphExtractService {
    config_receiver: tokio::sync::watch::Receiver<Config>,
    chars_receiver: tokio::sync::mpsc::Receiver<HashSet<char>>,
    rasterizer: crossfont::Rasterizer,

    font_key: Option<crossfont::FontKey>,
    current_font_size: Option<f32>,

    sender: tokio::sync::mpsc::Sender<ExtractionInfo>,

    // 過去にラスタライズした文字
    glyph_cache: HashSet<char>,

    // 遅延ラスタライズ
    lazy_string: Option<String>,
}

impl GlyphExtractService {
    pub fn new(
        config_receiver: tokio::sync::watch::Receiver<Config>,
        chars_receiver: tokio::sync::mpsc::Receiver<HashSet<char>>,
    ) -> (Self, tokio::sync::mpsc::Receiver<ExtractionInfo>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        (
            Self {
                config_receiver,
                chars_receiver,
                rasterizer: crossfont::Rasterizer::new().unwrap(),
                font_key: None,
                current_font_size: None,
                glyph_cache: Default::default(),
                sender,
                lazy_string: None,
            },
            receiver,
        )
    }

    pub async fn serve(mut self, mut token: renge::CancellationToken) {
        loop {
            tokio::select!(
            Ok(()) = self.config_receiver.changed() => {
                let config = self.config_receiver.borrow_and_update().clone();
                self.apply_config(config).await;
            },
            Some(string) = self.chars_receiver.recv() => self.extract(&string).await,
            _ = &mut token => break,
            else => break,
            )
        }
    }

    async fn extract(&mut self, str: &HashSet<char>) {
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

        // ラスタライズ
        let mut rasterized_glyph = HashMap::default();
        for c in str {
            // ラスタライズ済みなら何もしない
            if self.glyph_cache.contains(&c) {
                continue;
            }

            let Ok(glyph) = self.rasterizer.get_glyph(crossfont::GlyphKey {
                character: *c,
                font_key,
                size: crossfont::Size::new(font_size),
            }) else {
                // TODO: 豆腐対応
                continue;
            };

            rasterized_glyph.insert(*c, glyph);
            self.glyph_cache.insert(*c);
        }

        // 更新した文字を通知
        self.sender
            .send(ExtractionInfo {
                glyphs: rasterized_glyph,
            })
            .await
            .unwrap_or_default();
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

        // 遅延初期化分
        if let Some(lazy_str) = self.lazy_string.take() {
            self.extract(&lazy_str.chars().collect()).await;
        }
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
