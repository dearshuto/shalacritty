use std::time::Duration;

use tracing::instrument;

use crate::app::WindowSizeChangedEventArgs;

pub struct WindowSizeSendService {
    window_size_receiver: std::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,

    window_size_sender: Vec<tokio::sync::mpsc::Sender<WindowSizeChangedEventArgs>>,
}

impl WindowSizeSendService {
    pub fn new(receiver: std::sync::mpsc::Receiver<WindowSizeChangedEventArgs>) -> Self {
        Self {
            window_size_receiver: receiver,
            window_size_sender: Vec::default(),
        }
    }

    #[instrument]
    pub async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(20)) => {
                    if !self.try_apply_window_size().await {
                        break;
                    }
                },
                _ = &mut cancellation_token => break
            }
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs> {
        // 特に根拠はないが同期待ちでブロッキングしないようにある程度のバッファリングを確保
        let (sender, receiver) = tokio::sync::mpsc::channel(3);
        self.window_size_sender.push(sender);

        receiver
    }

    async fn try_apply_window_size(&mut self) -> bool {
        match self.window_size_receiver.try_recv() {
            Ok(args) => {
                // 変更通知が来ていたので再通知
                let fugures = self
                    .window_size_sender
                    .iter()
                    .map(|sender| sender.send(args.clone()));
                futures::future::join_all(fugures).await;
                return true;
            }
            Err(error) => match error {
                std::sync::mpsc::TryRecvError::Empty => return true,
                std::sync::mpsc::TryRecvError::Disconnected => return false,
            },
        }
    }
}

impl std::fmt::Debug for WindowSizeSendService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WindowSizeSendService")?;
        std::fmt::Result::Ok(())
    }
}

impl renge::Service for WindowSizeSendService {
    fn serve(
        self,
        cancellation_token: renge::CancellationToken,
    ) -> impl std::prelude::rust_2024::Future<Output = ()> + Send {
        self.serve(cancellation_token)
    }
}
