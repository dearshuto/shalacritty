use futures::FutureExt;

use crate::app::WindowSizeChangedEventArgs;

pub struct ShellService {
    receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    exit_receiver: tokio::sync::watch::Receiver<bool>,
}

impl ShellService {
    pub fn new(
        receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        exit_receiver: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            receiver,
            exit_receiver,
        }
    }

    pub async fn update_async(&mut self) -> Result<(), ()> {
        let result = tokio::select!(
            _args = self.receiver.recv().fuse() => {
        // ここでウィンドウサイズ変更によるあれこれを実装する
        Ok(())},
            _ = self.exit_receiver.changed().fuse() => Err(())
        );

        result
    }
}
