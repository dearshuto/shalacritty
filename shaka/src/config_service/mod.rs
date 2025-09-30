mod loader;

use loader::ConfigProxy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_font_size")]
    pub font_size: f32,
}

fn default_font_size() -> f32 {
    32.0
}

pub struct ConfigService {
    config_sender: tokio::sync::watch::Sender<Config>,
}

impl ConfigService {
    pub fn new() -> Self {
        let config = ConfigProxy::new().unwrap();
        let config_sender = tokio::sync::watch::Sender::new(config);
        Self { config_sender }
    }

    pub fn listen(&mut self) -> tokio::sync::watch::Receiver<Config> {
        self.config_sender.subscribe()
    }
}

impl renge::Service for ConfigService {
    async fn serve(self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
                _ = &mut cancellation_token => break,
                else => {},
            }
        }
    }
}
