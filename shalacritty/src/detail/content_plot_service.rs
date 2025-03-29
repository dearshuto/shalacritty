use tracing::instrument;

pub struct Diff {
    pub contents: Vec<RendarableContent>,
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

    diff_sender: tokio::sync::mpsc::Sender<Diff>,

    container: crate::detail::glyph_extract_service::Container,
}

impl ContentPlotService {
    pub fn new(
        contents_receiver: tokio::sync::mpsc::Receiver<(asura::ShellId, asura::Diff)>,
        container: crate::detail::glyph_extract_service::Container,
    ) -> (Self, tokio::sync::mpsc::Receiver<Diff>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        (
            Self {
                contents_receiver,
                diff_sender: sender,
                container,
            },
            receiver,
        )
    }

    #[instrument]
    pub async fn serve(mut self) {
        loop {
            tokio::select!(
                Some((id, diff)) = self.contents_receiver.recv() => self.calculate_diff(id, diff).await,
                else => break,
            );
        }
    }

    #[instrument]
    async fn calculate_diff(&mut self, _id: asura::ShellId, diff: asura::Diff) {
        let mut contents = Vec::default();

        let coord_table = self.container.read().await;

        // 差分検出範囲
        for diff_type in diff.content_diff {
            match diff_type {
                asura::DiffType::Add(content) => {
                    let (uv0, uv1) = if let Some(glyph) = coord_table.get(&content.content.code) {
                        (
                            nalgebra::Vector2::from(glyph.coord_range.top_left),
                            nalgebra::Vector2::from(glyph.coord_range.bottom_right),
                        )
                    } else {
                        // ラスタライズが追いついてなかったので適当なグリフを指定
                        (
                            nalgebra::Vector2::new(1.0, 1.0),
                            nalgebra::Vector2::new(1.0, 1.0),
                        )
                    };

                    let fg = content.content.fg;
                    contents.push(RendarableContent {
                        index: content.index,
                        transform: Default::default(),
                        fore_ground_color: [fg[0], fg[1], fg[2], 1.0],
                        uv0,
                        uv1,
                    });
                }
                asura::DiffType::Update(_) => {}
                asura::DiffType::Remove(_index) => {}
            };
        }

        // 使い終わったら早めにドロップしてロックを解放する
        drop(coord_table);

        let diff = Diff { contents };
        self.diff_sender.send(diff).await.unwrap_or_default();
    }
}

impl std::fmt::Debug for ContentPlotService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ContentPlotService")?;
        std::fmt::Result::Ok(())
    }
}
