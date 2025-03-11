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
