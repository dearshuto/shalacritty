pub struct BinarizedData {
    pub chunks: Vec<Vec<u8>>,
    pub count: Option<usize>,
}

pub struct BinarizeService {
    diff_receiver: tokio::sync::mpsc::Receiver<crate::detail::Diff>,
    sender: tokio::sync::mpsc::Sender<BinarizedData>,
}

impl BinarizeService {
    pub fn new(
        diff_receiver: tokio::sync::mpsc::Receiver<crate::detail::Diff>,
    ) -> (Self, tokio::sync::mpsc::Receiver<BinarizedData>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        (
            Self {
                diff_receiver,
                sender,
            },
            receiver,
        )
    }

    pub async fn serve(mut self) {
        while let Some(diff) = self.diff_receiver.recv().await {
            self.apply_diff(diff).await;
        }
    }

    async fn apply_diff(&self, diff: crate::detail::Diff) {
        let mut chunks = Vec::default();

        // 番兵
        let mut expected_diff = usize::MAX;
        for (index, content) in diff.contents.into_iter().enumerate() {
            // index はイテレーターのインデックスなので確定で 0 始まりの連番
            // content のインデックスは飛ぶことがあることを考慮すると、
            // 常に content.index >= index が成立するので、
            // content.index - index で値が溢れることはない
            let current_diff = content.index - index;

            if current_diff == expected_diff {
                // 連番した文字の差分なので連続したバイト列として末尾に付与
                let chunk: &mut Vec<u8> = chunks.last_mut().unwrap();
                // TODO: 適切な構造体に変換する
                chunk.extend_from_slice(&[0u8]);
                continue;
            }

            // 連番が終わった
            expected_diff = current_diff;
            // TODO: 適切な構造体に変換する
            chunks.push(vec![0u8]);
        }

        let data = BinarizedData {
            chunks,
            count: diff.content_count,
        };
        self.sender.send(data).await.unwrap();
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn it_works() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .build()
            .unwrap();

        let (diff_sender, diff_receiver) = tokio::sync::mpsc::channel(1);
        let (binarize_service, mut receiver) = BinarizeService::new(diff_receiver);

        let _service_task = runtime.spawn(async move {
            binarize_service.serve().await;
        });

        diff_sender
            .blocking_send(crate::detail::Diff {
                contents: Vec::default(),
                content_count: Some(19),
            })
            .unwrap();

        let binarized_data = receiver.blocking_recv().unwrap();
        assert_eq!(binarized_data.count, Some(19));
    }
}
