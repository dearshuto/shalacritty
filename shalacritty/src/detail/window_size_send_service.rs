use crate::app::WindowSizeChangedEventArgs;

pub struct WindowSizeSendService {
    window_size_receiver: std::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,

    polling_event_receiver: tokio::sync::mpsc::Receiver<()>,

    window_size_sender: Vec<tokio::sync::mpsc::Sender<WindowSizeChangedEventArgs>>,
}

impl WindowSizeSendService {
    pub fn new(
        receiver: std::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        polling_event_receiver: tokio::sync::mpsc::Receiver<()>,
    ) -> Self {
        Self {
            window_size_receiver: receiver,
            polling_event_receiver,
            window_size_sender: Vec::default(),
        }
    }

    pub async fn serve(mut self) {
        // TODO: デッドロックする
        // ウィンドウサイズの変更をポーリングで監視
        while let Some(_) = self.polling_event_receiver.recv().await {
            match self.window_size_receiver.try_recv() {
                Ok(args) => {
                    // 変更通知が来ていたので再通知
                    let fugures = self
                        .window_size_sender
                        .iter()
                        .map(|sender| sender.send(args.clone()));
                    futures::future::join_all(fugures).await;
                    continue;
                }
                Err(error) => match error {
                    std::sync::mpsc::TryRecvError::Empty => continue,
                    std::sync::mpsc::TryRecvError::Disconnected => break,
                },
            }
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs> {
        // 特に根拠はないが同期待ちでブロッキングしないようにある程度のバッファリングを確保
        let (sender, receiver) = tokio::sync::mpsc::channel(3);
        self.window_size_sender.push(sender);

        receiver
    }
}
