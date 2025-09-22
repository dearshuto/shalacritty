mod loader;

use std::path::PathBuf;

use loader::ConfigProxy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {}

pub struct ConfigService {
    config_sender: tokio::sync::watch::Sender<Config>,
}

impl ConfigService {
    pub fn new() -> Self {
        let config = ConfigProxy::new().unwrap();
        let (config_sender, config_receiver) = tokio::sync::watch::channel(config);
        Self { config_sender }
    }

    pub fn listen(&mut self) -> tokio::sync::watch::Receiver<Config> {
        self.config_sender.subscribe()
    }
}
