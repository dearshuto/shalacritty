use std::collections::{HashMap, HashSet};

use tracing::instrument;

use crate::app::WindowSizeChangedEventArgs;

use super::glyph_extract_service::Glyph;

pub struct Diff {
    pub contents: Vec<RendarableContent>,
    pub content_count: Option<usize>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RendarableContent {
    pub index: usize,
    pub transform: nalgebra::Matrix3x2<f32>,
    pub fore_ground_color: [f32; 4],
    pub uv0: nalgebra::Vector2<f32>,
    pub uv1: nalgebra::Vector2<f32>,
}

pub struct ContentPlotService {
    contents_receiver: tokio::sync::mpsc::Receiver<(asura::ShellId, asura::Diff)>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    rasterized_chars_receiver: tokio::sync::mpsc::Receiver<String>,

    diff_sender: tokio::sync::mpsc::Sender<Diff>,

    container: crate::detail::glyph_extract_service::Container,

    size: Option<(u32, u32)>,

    miss_contents: Vec<asura::DiffContent>,
}

impl ContentPlotService {
    pub fn new(
        contents_receiver: tokio::sync::mpsc::Receiver<(asura::ShellId, asura::Diff)>,
        window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        rasterized_chars_receiver: tokio::sync::mpsc::Receiver<String>,
        container: crate::detail::glyph_extract_service::Container,
    ) -> (Self, tokio::sync::mpsc::Receiver<Diff>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        (
            Self {
                contents_receiver,
                window_size_receiver,
                rasterized_chars_receiver,
                diff_sender: sender,
                container,
                size: None,
                miss_contents: Vec::default(),
            },
            receiver,
        )
    }

    #[instrument]
    pub async fn serve(mut self) {
        // ウィンドウサイズは必須なので最低でも初回の通知は待つ
        if let Some(args) = self.window_size_receiver.recv().await {
            self.apply_window_size(args).await;
        } else {
            // なんらかの理由で送信元が終了していたら続行不可能なので処理を抜ける
            return;
        }

        loop {
            tokio::select!(
            Some((id, diff)) = self.contents_receiver.recv() => self.calculate_diff(id, diff).await,
            Some(args) = self.window_size_receiver.recv() => self.apply_window_size(args).await,
            Some(rasterized_chars) = self.rasterized_chars_receiver.recv() => self.apply_rasterized_chars(&rasterized_chars).await,
            else => break,
            );
        }
    }

    async fn apply_window_size(&mut self, args: WindowSizeChangedEventArgs) {
        self.size = Some((args.width, args.height));
    }

    async fn apply_rasterized_chars(&mut self, chars: &str) {
        // ウィンドウサイズが不明だと計算できない
        let size = self.size.unwrap();

        let glyph_table = self.container.read().await;
        let chars = HashSet::<char>::from_iter(chars.chars());

        // 要素を pop しながら走査するので後ろから調べる
        let mut contents = Vec::default();
        for index in (0..self.miss_contents.len()).rev() {
            // 新たにラスタライズされた文字に含まれているか
            let miss_content_code = &self.miss_contents[index].content.code;
            if !chars.contains(&miss_content_code) {
                // まだラスタライズできてなかった
                continue;
            }

            // 描画コンテンツに変換する
            // ラスタライズの完了チェックはしてあるので必ず成功することを期待してノーチェックで unwrap()
            let miss_content = self.miss_contents.remove(index);
            let content = Self::into_rendarable_content(&miss_content, &glyph_table, size).unwrap();

            contents.push(content);
        }

        // 使い終わったら早めにドロップしてロックを解放する
        drop(glyph_table);

        // 新たにラスタライズに成功した要素を通知
        let diff = Diff {
            contents,
            content_count: None,
        };
        self.diff_sender.send(diff).await.unwrap_or_default();
    }

    #[instrument]
    async fn calculate_diff(&mut self, _id: asura::ShellId, diff: asura::Diff) {
        // ウィンドウサイズが不明だと計算できない
        let size = self.size.unwrap();

        let mut contents = Vec::default();
        let mut miss_contents = Vec::default();
        let coord_table = self.container.read().await;

        // 差分検出範囲
        for diff_type in diff.content_diff {
            match diff_type {
                asura::DiffType::Add(content) => {
                    if let Some(content) =
                        Self::into_rendarable_content(&content, &coord_table, size)
                    {
                        contents.push(content);
                    } else {
                        miss_contents.push(content);
                    }
                }
                asura::DiffType::Update(content) => {
                    if let Some(content) =
                        Self::into_rendarable_content(&content, &coord_table, size)
                    {
                        contents.push(content);
                    } else {
                        miss_contents.push(content);
                    }
                }
                asura::DiffType::Remove(_index) => {}
            };
        }

        // 使い終わったら早めにドロップしてロックを解放する
        drop(coord_table);

        let diff = Diff {
            contents,
            content_count: diff.content_count,
        };
        self.diff_sender.send(diff).await.unwrap_or_default();
    }

    fn into_rendarable_content(
        diff_content: &asura::DiffContent,
        glyph_table: &HashMap<char, Glyph>,
        window_size: (u32, u32),
    ) -> Option<RendarableContent> {
        let Some(glyph) = glyph_table.get(&diff_content.content.code) else {
            // ラスタライズが追いついてなかった
            return None;
        };

        let (uv0, uv1, width, height, left, top) = (
            nalgebra::Vector2::from(glyph.coord_range.top_left),
            nalgebra::Vector2::from(glyph.coord_range.bottom_right),
            glyph.width,
            glyph.height,
            glyph.left,
            glyph.top,
        );

        // ピクセル座標で 1x1 の四角形をフォントのサイズにスケール
        let local_pixel_scale_matrix = nalgebra::Matrix3::new_nonuniform_scaling(
            &nalgebra::Vector2::new(width as f32, height as f32),
        );

        // ピクセル座標で表示位置をずらす
        let local_pixel_translate_matrix = nalgebra::Matrix3::new_translation(
            &nalgebra::Vector2::new(left as f32, (32 - top) as f32),
        );

        // ピクセル座標を [0, 1] 空間に変換する行列
        // フレームバッファーのサイズで変わる
        // 文字間を開けて見栄えを整えるために文字サイズを 0.6 倍している
        let normalized_matrix = nalgebra::Matrix3::new_nonuniform_scaling(&nalgebra::Vector2::new(
            0.6f32 / window_size.0 as f32,
            0.6f32 / window_size.1 as f32,
        ));

        // [0, 1] => [-1, 1]
        let view_matrix = nalgebra::Matrix3::new_translation(&nalgebra::Vector2::new(-1.0, -1.0))
            * nalgebra::Matrix3::new_scaling(2.0);

        // 画面上に配置
        let offset_matrix = nalgebra::Matrix3::new_translation(
            &(nalgebra::Vector2::new(
                diff_content.content.x as f32 / (window_size.0 as f32 / 16.0),
                diff_content.content.y as f32 / (window_size.1 as f32 / 16.0),
            )),
        );

        let transform_matrix = view_matrix
            * offset_matrix
            * normalized_matrix
            * local_pixel_translate_matrix
            * local_pixel_scale_matrix;

        let fg = diff_content.content.fg;
        Some(RendarableContent {
            index: diff_content.index,
            transform: transform_matrix.transpose().remove_column(2),
            fore_ground_color: [fg[0], fg[1], fg[2], 1.0],
            uv0,
            uv1,
        })
    }
}

impl std::fmt::Debug for ContentPlotService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ContentPlotService")?;
        std::fmt::Result::Ok(())
    }
}
