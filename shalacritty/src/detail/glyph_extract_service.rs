use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crossfont::{FontDesc, Rasterize, Slant, Style, Weight};
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::instrument;

use crate::{gfx::GlyphTexturePatch, Config};

use super::{CoordRange, GlyphWriterEx};

pub struct Glyph {
    pub coord_range: CoordRange,
    pub width: u32,
    pub height: u32,
    pub top: i32,
    pub left: i32,
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
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
    sender: tokio::sync::mpsc::Sender<Vec<GlyphTexturePatch>>,
    extract_senders: Vec<tokio::sync::mpsc::Sender<String>>,
}

impl GlyphExtractService {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        string_receiver: tokio::sync::mpsc::Receiver<String>,
    ) -> (Self, tokio::sync::mpsc::Receiver<Vec<GlyphTexturePatch>>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        (
            Self {
                config_receiver,
                receiver: string_receiver,
                rasterizer: crossfont::Rasterizer::new().unwrap(),
                glyph_writer: GlyphWriterEx::new(32, 32),
                font_key: None,
                current_font_size: None,
                table: Default::default(),
                sender,
                extract_senders: Vec::default(),
            },
            receiver,
        )
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

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<String> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.extract_senders.push(sender);
        return receiver;
    }

    pub fn share_glyph_container(&self) -> Container {
        Container {
            table: self.table.clone(),
        }
    }

    #[instrument(skip(self), fields(str = str))]
    async fn extract(&mut self, str: &str) {
        // フォントサイズが不明な状態だとなにもしない
        let Some(font_size) = self.current_font_size else {
            return;
        };

        // FontKey がなければグリフが決まらないのでなにもしない
        let Some(font_key) = self.font_key else {
            return;
        };

        let chars = {
            // 重複排除
            let mut str: HashSet<char> = str.chars().collect();

            // ロック区間は最短で
            let table = self.table.read().await;

            // キャッシュに存在しない文字をしぼりこむ
            str.retain(|character| !table.contains_key(&character));

            str
        };

        // グリフを更新
        let (patches, rasterized_chars) = {
            let mut table = self.table.write().await;
            Self::update_glyph_table(
                &mut table,
                &mut self.glyph_writer,
                &mut self.rasterizer,
                chars,
                font_size,
                font_key,
            )
            .await
        };

        // 更新した文字を通知
        for sender in &self.extract_senders {
            sender.send(rasterized_chars.clone()).await.unwrap();
        }

        // 差分を通知
        if !self.sender.is_closed() {
            self.sender.send(patches).await.unwrap_or_default();
        }
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
        let chars = { self.table.read().await.keys().copied().collect() };

        // グリフを再抽出
        let (patches, rasterized_chars) = {
            let mut table = self.table.write().await;
            Self::update_glyph_table(
                &mut table,
                &mut self.glyph_writer,
                &mut self.rasterizer,
                chars,
                config.font_size,
                *self.font_key.as_ref().unwrap(),
            )
            .await
        };

        // 抽出した文字を通知
        for sender in &self.extract_senders {
            sender.send(rasterized_chars.clone()).await.unwrap();
        }

        // 差分を通知
        if !self.sender.is_closed() {
            self.sender.send(patches).await.unwrap_or_default();
        }
    }

    #[instrument(skip(table, glyph_writer, rasterizer))]
    async fn update_glyph_table(
        table: &mut HashMap<char, Glyph>,
        glyph_writer: &mut GlyphWriterEx,
        rasterizer: &mut crossfont::Rasterizer,
        chars: HashSet<char>,
        font_size: f32,
        font_key: crossfont::FontKey,
    ) -> (Vec<GlyphTexturePatch>, String) {
        let mut string = String::new();
        let mut patches = Vec::default();
        for c in chars {
            // すでに処理ずみならなにもしない
            if table.contains_key(&c) {
                continue;
            }

            // ラスタライズ
            let Ok(glyph) = rasterizer.get_glyph(crossfont::GlyphKey {
                character: c,
                font_key,
                size: crossfont::Size::new(font_size),
            }) else {
                // TODO: 豆腐対応
                continue;
            };

            string.push(c);

            let bytes: Vec<u8> = match glyph.buffer {
                crossfont::BitmapBuffer::Rgb(items) => items.chunks(3).map(|rgb| rgb[0]).collect(),
                crossfont::BitmapBuffer::Rgba(items) => items.chunks(4).map(|rgb| rgb[0]).collect(),
            };

            // 2 次元に配置
            glyph_writer.allocate(c);

            let coord_range = glyph_writer.get_coord_range(c).unwrap();
            table.insert(
                c,
                Glyph {
                    bytes: bytes.clone(),
                    coord_range,
                    width: glyph.width as u32,
                    height: glyph.height as u32,
                    left: glyph.left,
                    top: glyph.top,
                },
            );

            let pixel_range = glyph_writer.get_pixel_range(c).unwrap();
            patches.push(GlyphTexturePatch {
                offset_x: pixel_range.top_left[0],
                offset_y: pixel_range.top_left[1],
                width: glyph.width as u32,
                height: glyph.height as u32,
                pixels: bytes,
            });
        }

        (patches, string)
    }

    #[instrument(skip(rasterizer))]
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

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn it_works() {
        let (config_sender, config_receiver) = tokio::sync::mpsc::channel(1);
        let (string_sender, string_receiver) = tokio::sync::mpsc::channel(1);
        let (glyph_extract_service, mut receiver) =
            GlyphExtractService::new(config_receiver, string_receiver);

        let task = tokio::spawn(async { glyph_extract_service.serve().await });

        config_sender
            .send(Config {
                font_size: 32.0,
                image: Default::default(),
                image_alpha: Default::default(),
                background: Default::default(),
            })
            .await
            .unwrap();
        receiver.recv().await.unwrap();

        string_sender.send(String::from("ABC")).await.unwrap();

        if let Some(patch) = receiver.recv().await {
            for (index, patch) in patch.iter().enumerate() {
                let image = image::GrayImage::from_vec(
                    patch.width,
                    patch.height,
                    patch.pixels().iter().copied().collect(),
                )
                .unwrap();
                image.save(format!("{}.png", index)).unwrap();
            }
        }

        task.await.unwrap();
    }
}
