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
    contents_receiver: tokio::sync::mpsc::Receiver<(asura::ShellId, Vec<asura::Content>)>,

    diff_sender: tokio::sync::mpsc::Sender<Diff>,

    cache: Vec<asura::Content>,

    container: crate::detail::glyph_extract_service::Container,
}

impl ContentPlotService {
    pub fn new(
        contents_receiver: tokio::sync::mpsc::Receiver<(asura::ShellId, Vec<asura::Content>)>,
        container: crate::detail::glyph_extract_service::Container,
    ) -> (Self, tokio::sync::mpsc::Receiver<Diff>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        (
            Self {
                contents_receiver,
                diff_sender: sender,
                cache: Vec::default(),
                container,
            },
            receiver,
        )
    }

    #[instrument]
    pub async fn serve(mut self) {
        loop {
            tokio::select!(
                Some((id, contents)) = self.contents_receiver.recv() => self.calculate_diff(id, contents).await,
                else => break,
            );
        }
    }

    #[instrument]
    async fn calculate_diff(&mut self, _id: asura::ShellId, new_contents: Vec<asura::Content>) {
        // インデックスで操作できるようにキャッシュサイズを調節する
        // この際に new_contents の方が大きければ確定で要素の追加なので newer_index を保持しておく
        //
        // イテレーターで処理しようとするとライフタイムの制約に引っかかる
        let newer_index = if new_contents.len() == self.cache.len() {
            // なにもしなくてよい
            new_contents.len()
        } else if new_contents.len() < self.cache.len() {
            // キャッシュサイズを切り詰める
            self.cache.truncate(new_contents.len());
            new_contents.len()
        } else {
            // キャッシュサイズより伸びた分は全て追加扱い
            let newer_index = self.cache.len();

            // キャッシュサイズを拡張
            // 適当な値を代入する必要があるので、ここの分岐に突入したときに必ず存在する 0 番目の値で埋めておく
            debug_assert!(!new_contents.is_empty());
            self.cache
                .resize(new_contents.len(), new_contents[0].clone());
            newer_index
        };

        // 新旧の値を先頭から順次比較していく
        // インデックス操作するのでここに到達した時点でキャッシュとサイズが一致していることを期待
        debug_assert_eq!(new_contents.len(), self.cache.len());
        let mut contents = Vec::default();

        let coord_table = self.container.read().await;

        // 差分検出範囲
        for index in 0..new_contents.len() {
            let new = &new_contents[index];
            let old = &self.cache[index];

            // 新旧どちらも値がある場合は差分検出が必要
            if index < newer_index && new == old {
                continue;
            }

            if let Some(glyph) = coord_table.get(&new.code) {
                contents.push(RendarableContent {
                    index,
                    transform: Default::default(),
                    fore_ground_color: Default::default(),
                    uv0: nalgebra::Vector2::from(glyph.coord_range.top_left),
                    uv1: nalgebra::Vector2::from(glyph.coord_range.bottom_right),
                });

                self.cache[index] = new.clone();
            } else {
                // ラスタライズが追いついてなかったので適当なグリフを指定
                self.cache[index] = asura::Content {
                    code: ' ',
                    x: 0,
                    y: 0,
                    fg: [0.0; 3],
                };
            }
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
