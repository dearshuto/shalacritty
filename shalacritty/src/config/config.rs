use std::{
    path::PathBuf,
    sync::{Arc, LockResult, Mutex, MutexGuard, RwLock},
};

use notify::Watcher;
use serde::{Deserialize, Serialize};

use super::detail::EventHandlerAdapter;

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
    // ConfigService は各種オブジェクトに共有することを想定するので Send + Sync
    #[allow(dead_code)]
    watcher: Arc<dyn notify::Watcher + Send + Sync>,
    #[allow(dead_code)]
    path: Arc<PathBuf>,
    config: Arc<Mutex<Config>>,

    // 設定の更新を通知する sender たち
    senders: Arc<RwLock<Vec<std::sync::mpsc::Sender<Config>>>>,

    // 設定更新通知のタスクを終了するシグナルを通知する sender
    config_watcher_close_signal_sender: std::sync::mpsc::Sender<()>,

    // 設定更新を通知するタスク
    config_notify_task_handle: tokio::task::JoinHandle<()>,
}

impl ConfigService {
    pub fn new(runtime: Arc<tokio::runtime::Runtime>) -> Self {
        // コンフィグ置き場。なければ作る。
        let mut config_path = super::util::create_config_directory();
        let config = if config_path.exists() {
            config_path.push("config.toml");
            super::util::load_config(&config_path.to_path_buf())
        } else {
            std::fs::DirBuilder::new().create(&config_path).unwrap();
            Config::default()
        };

        let (sender, receiver) = std::sync::mpsc::channel();
        let config = Arc::new(Mutex::new(config));
        let mut watcher = notify::RecommendedWatcher::new(
            EventHandlerAdapter::new(sender),
            notify::Config::default(),
        )
        .unwrap();
        watcher
            .watch(&config_path, notify::RecursiveMode::Recursive)
            .unwrap();

        //
        let (config_watcher_close_signal_sender, config_watcher_close_signal_receiver) =
            std::sync::mpsc::channel();
        let senders = Arc::new(RwLock::new(
            Vec::<std::sync::mpsc::Sender<Config>>::default(),
        ));
        let local = Arc::clone(&senders);
        let config_notify_task_handle = runtime.spawn(async move {
            while let Err(_) = config_watcher_close_signal_receiver.try_recv() {
                let Ok(config) = receiver.try_recv() else {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    continue;
                };

                let senders = local.read().unwrap();

                for sender in senders.iter() {
                    sender.send(config.clone()).unwrap();
                }
            }
        });

        Self {
            watcher: Arc::new(watcher),
            path: Arc::new(config_path),
            config,
            senders,
            config_watcher_close_signal_sender,
            config_notify_task_handle,
        }
    }

    pub fn read(&self) -> LockResult<MutexGuard<Config>> {
        self.config.lock()
    }

    pub fn listen(&mut self) -> std::sync::mpsc::Receiver<Config> {
        let (sender, receiver) = std::sync::mpsc::channel();

        let mut senders = self.senders.write().unwrap();
        senders.push(sender);

        receiver
    }
}

impl Drop for ConfigService {
    fn drop(&mut self) {
        // 設定ファイルの通知タスクを止める要求を出す
        self.config_watcher_close_signal_sender.send(()).unwrap();

        // タスクが完了するまで待つ
        while !self.config_notify_task_handle.is_finished() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // 解放した方がキレイだけどしなくてもよさそう
        // self.watcher.unwatch(&self.path).unwrap();
    }
}
