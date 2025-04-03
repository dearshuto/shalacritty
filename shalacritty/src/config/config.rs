use serde::{Deserialize, Serialize};

use super::detail::EventHandler;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub image: String,

    #[serde(default = "default_font_size")]
    pub font_size: f32,

    #[serde(default = "default_image_alpha")]
    pub image_alpha: f32,

    #[serde(default)]
    pub background: Background,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Background {
    #[serde(default = "default_clear_color")]
    pub clear_color: [f32; 4],

    #[serde(default)]
    pub path: Vec<String>,

    #[serde(default)]
    pub enhance: Vec<f32>,
}

fn default_font_size() -> f32 {
    32.0
}

fn default_image_alpha() -> f32 {
    1.0
}

fn default_clear_color() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

pub struct ConfigService {
    // 破棄の順番が大事なので宣言順が大事
    // まずレシーバーを破棄してから本体である EventHandler を破棄すること
    event_handler_receiver: tokio::sync::watch::Receiver<Config>,
    #[allow(unused)]
    event_handler: EventHandler,
    // 順番大事ここまで
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

pub fn watch() -> (Instance, tokio::sync::watch::Receiver<Config>) {
    // コンフィグ置き場。なければ作る。
    let config_path = super::util::create_config_directory();

    let (event_handler, event_receiver) = EventHandler::new(config_path);
    (
        Instance {
            handler: event_handler,
        },
        event_receiver,
    )
}

pub struct Instance {
    #[allow(unused)]
    handler: EventHandler,
}

pub struct ConfigServiceEx {
    receiver: tokio::sync::watch::Receiver<Config>,
    senders: Vec<tokio::sync::mpsc::Sender<Config>>,
}

impl ConfigServiceEx {
    pub fn new(receiver: tokio::sync::watch::Receiver<Config>) -> Self {
        Self {
            receiver,
            senders: Vec::default(),
        }
    }

    pub async fn serve(mut self) {
        // 初期値の通知
        for sender in &self.senders {
            let config = self.receiver.borrow().clone();
            sender.send(config).await.unwrap();
        }

        // 以降は変更があったら通知
        while let Ok(_) = self.receiver.changed().await {
            for sender in &self.senders {
                let config = self.receiver.borrow().clone();
                sender.send(config).await.unwrap();
            }
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<Config> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        // 初期値を積んでおく
        sender
            .blocking_send(self.receiver.borrow().clone())
            .unwrap();

        self.senders.push(sender);

        receiver
    }
}
