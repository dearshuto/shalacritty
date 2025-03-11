use crate::app::WindowSizeChangedEventArgs;

pub struct ShellService {
    receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
}

impl ShellService {
    pub fn new(receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>) -> Self {
        Self { receiver }
    }

    pub async fn update_async(&mut self) -> Result<(), ()> {
        let Some(_) = self.receiver.recv().await else {
            // sender 側がドロップしていた
            return Err(());
        };

        Ok(())
    }
}
