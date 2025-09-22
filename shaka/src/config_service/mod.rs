use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {}

pub struct ConfigService {
    receiver: tokio::sync::watch::Receiver<Config>,
    senders: Vec<tokio::sync::mpsc::Sender<Config>>,
}

impl ConfigService {
    pub fn new() -> Self {
        // コンフィグ置き場。なければ作る。
        let config_path = super::util::create_config_directory();

        let (event_handler, event_receiver) = EventHandler::new(config_path);

        Self {
            event_handler,
            event_handler_receiver: event_receiver,
        }
    }

    pub fn read(&self) -> Config {
        // MEMO: できれば参照で返したい
        self.event_handler_receiver.borrow().clone()
    }

    pub fn listen(&mut self) -> tokio::sync::watch::Receiver<Config> {
        self.event_handler_receiver.clone()
    }
}
