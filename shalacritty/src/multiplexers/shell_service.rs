use crate::{app::WindowSizeChangedEventArgs, Config};

pub struct ShellService {
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
}

impl ShellService {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    ) -> Self {
        Self {
            config_receiver,
            receiver,
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
                Some(config) = self.config_receiver.recv() => self.apply_config(config),
                Some(args) = self.receiver.recv() => self.apply_window_size(args),
                else => break,
            );
        }
    }

    fn apply_config(&mut self, _config: Config) {
        //
    }

    fn apply_window_size(&mut self, _args: WindowSizeChangedEventArgs) {
        //
    }
}
