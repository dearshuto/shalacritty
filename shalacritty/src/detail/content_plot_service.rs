use crate::Config;

pub struct ContentPlotService {
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
}

impl ContentPlotService {
    pub fn new(config_receiver: tokio::sync::mpsc::Receiver<Config>) -> Self {
        Self { config_receiver }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
                Some(_config) = self.config_receiver.recv() => {},
                else => break,
            );
        }
    }
}
