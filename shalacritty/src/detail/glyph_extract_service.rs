use std::{collections::HashMap, sync::Arc};

use crossfont::{FontDesc, Rasterize, Slant, Style, Weight};
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::instrument;

use crate::Config;

use super::{CoordRange, GlyphWriterEx};

pub struct Glyph {
    pub coord_range: CoordRange,
    pub bytes: Vec<u8>,
}

pub struct Container {
    table: Arc<RwLock<HashMap<char, Glyph>>>,
}

impl Container {
    pub async fn read(&self) -> RwLockReadGuard<HashMap<char, Glyph>> {
        self.table.read().await
    }
}

pub struct GlyphExtractService {
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    receiver: tokio::sync::mpsc::Receiver<String>,
    rasterizer: crossfont::Rasterizer,
    glyph_writer: GlyphWriterEx,

    font_key: Option<crossfont::FontKey>,
    current_font_size: Option<f32>,

    table: Arc<RwLock<HashMap<char, Glyph>>>,
}

impl GlyphExtractService {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        string_receiver: tokio::sync::mpsc::Receiver<String>,
    ) -> Self {
        Self {
            config_receiver,
            receiver: string_receiver,
            rasterizer: crossfont::Rasterizer::new().unwrap(),
            glyph_writer: GlyphWriterEx::new(8, 8),
            font_key: None,
            current_font_size: None,
            table: Default::default(),
        }
    }

    #[instrument]
    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(config) = self.config_receiver.recv() => self.apply_config(config).await,
            Some(string) = self.receiver.recv() => self.extract(&string).await,
            else => break,
            )
        }
    }

    pub fn share_glyph_container(&self) -> Container {
        Container {
            table: self.table.clone(),
        }
    }

    #[instrument]
    async fn extract(&mut self, str: &str) {
        // フォントサイズが不明な状態だとなにもしない
        let Some(font_size) = self.current_font_size else {
            return;
        };

        // FontKey がなければグリフが決まらないのでなにもしない
        let Some(font_key) = self.font_key else {
            return;
        };

        let chars: Vec<char> = {
            // ロック区間は最短で
            let table = self.table.read().await;

            // キャッシュに存在しない文字をしぼりこむ
            str.chars()
                .filter(|character| !table.contains_key(&character))
                .collect()
        };

        // グリフを更新
        let mut table = self.table.write().await;
        Self::update_glyph_table(
            &mut table,
            &mut self.glyph_writer,
            &mut self.rasterizer,
            chars.into_iter(),
            font_size,
            font_key,
        )
        .await;
    }

    #[instrument]
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

        // フォントサイズのキャッシュを更新
        self.current_font_size = Some(config.font_size);

        // フォントサイズが変更になったので、全てラスタライズしなおす
        // ラスタライズが必要な文字を抽出する処理は早めにロックを解放したいのでスコープを作る
        let chars: Vec<char> = { self.table.read().await.keys().copied().collect() };

        // グリフを再抽出
        let mut table = self.table.write().await;
        Self::update_glyph_table(
            &mut table,
            &mut self.glyph_writer,
            &mut self.rasterizer,
            chars.into_iter(),
            config.font_size,
            *self.font_key.as_ref().unwrap(),
        )
        .await;
    }

    async fn update_glyph_table(
        table: &mut HashMap<char, Glyph>,
        glyph_writer: &mut GlyphWriterEx,
        rasterizer: &mut crossfont::Rasterizer,
        chars: impl Iterator<Item = char>,
        font_size: f32,
        font_key: crossfont::FontKey,
    ) {
        let key_values = chars.into_iter().map(|character| {
            let glyph = rasterizer
                .get_glyph(crossfont::GlyphKey {
                    character,
                    font_key,
                    size: crossfont::Size::new(font_size),
                })
                .unwrap(); // TODO: 不正なフォント対応
            (character, glyph)
        });

        for (key, value) in key_values {
            let bytes = match value.buffer {
                crossfont::BitmapBuffer::Rgb(items) => items,
                crossfont::BitmapBuffer::Rgba(items) => items,
            };

            // 2 次元に配置
            glyph_writer.allocate(key);
            let coord_range = glyph_writer.get_coord_range(key).unwrap();

            table.insert(key, Glyph { bytes, coord_range });
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
