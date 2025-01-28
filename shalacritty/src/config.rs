use std::{
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, LockResult, Mutex, MutexGuard, RwLock},
};

use notify::Watcher;
use serde::{Deserialize, Serialize};

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
    24.0
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
        let mut config_path = create_config_directory();
        let config = if config_path.exists() {
            config_path.push("config.toml");
            load_config(&config_path.to_path_buf())
        } else {
            std::fs::DirBuilder::new().create(&config_path).unwrap();
            Config::default()
        };

        let (sender, receiver) = std::sync::mpsc::channel();
        let config = Arc::new(Mutex::new(config));
        let mut watcher = notify::RecommendedWatcher::new(
            EventHandler {
                sender,
                config: config.clone(),
            },
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

struct EventHandler {
    config: Arc<Mutex<Config>>,
    sender: std::sync::mpsc::Sender<Config>,
}

impl notify::EventHandler for EventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let Ok(e) = event else {
            return;
        };

        match e.kind {
            // notify::EventKind::Any => todo!(),
            // notify::EventKind::Access(_) => todo!(),
            notify::EventKind::Create(kind) => {
                if kind != notify::event::CreateKind::File {
                    return;
                }

                // 定義ファイルが作成されたので読み込む
                for path in &e.paths {
                    println!("{:?}: {:?}", e.kind, path);
                    let _file = std::fs::File::open(path).unwrap();
                    self.sender.send(Config::default()).unwrap();
                }
            }
            notify::EventKind::Modify(kind) => {
                let notify::event::ModifyKind::Data(_) = kind else {
                    return;
                };

                // 定義ファイルが更新されたので読み込む
                for path in &e.paths {
                    let config = load_config(path);
                    *self.config.lock().unwrap() = config.clone();
                    self.sender.send(config).unwrap();
                }
            }
            // notify::EventKind::Remove(_) => todo!(),
            // notify::EventKind::Other => todo!(),
            _ => {}
        }
    }
}

fn load_config(path: &Path) -> Config {
    let mut file = std::fs::File::open(path).unwrap();
    let mut str: String = String::new();
    file.read_to_string(&mut str).ok().unwrap();
    let mut config: Config = toml::from_str(&str).unwrap();

    // 画像パスは設定ファイルからの相対パスにする
    let mut image_path = path.to_path_buf();
    image_path.pop();
    image_path.push(config.image);
    config.image = image_path.to_str().unwrap().to_string();

    for path in &mut config.background.path {
        image_path.push(&path);
        *path = image_path.to_str().unwrap().to_string();
        image_path.pop();
    }

    config
}

fn create_config_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    let home_directory = std::env::var("APPDATA").unwrap();

    #[cfg(target_os = "macos")]
    let home_directory = std::env::var("HOME").unwrap();

    #[cfg(target_os = "linux")]
    let home_directory = std::env::var("HOME").unwrap();

    let mut config_directory_path = PathBuf::from_str(&home_directory).unwrap();
    config_directory_path.push(".config");
    config_directory_path.push("shalacritty");

    config_directory_path
}
