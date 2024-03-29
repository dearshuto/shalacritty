use std::{
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

use notify::Watcher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub image: String,

    #[serde(default = "default_image_alpha")]
    pub image_alpha: f32,

    #[serde(default)]
    pub background: Background,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Background {
    #[serde(default)]
    pub clear_color: [f32; 4],

    #[serde(default)]
    pub path: Vec<String>,
}

fn default_image_alpha() -> f32 {
    1.0
}

pub struct ConfigService {
    // ConfigService は各種オブジェクトに共有することを想定するので Send + Sync
    #[allow(dead_code)]
    watcher: Arc<dyn notify::Watcher + Send + Sync>,
    #[allow(dead_code)]
    path: Arc<PathBuf>,
    config: Arc<Mutex<Config>>,
}

impl ConfigService {
    pub fn new() -> Self {
        // コンフィグ置き場。なければ作る。
        let mut config_path = create_config_directory();
        let config = if config_path.exists() {
            config_path.push("config.toml");
            load_config(&config_path.to_path_buf())
        } else {
            std::fs::DirBuilder::new().create(&config_path).unwrap();
            Config::default()
        };

        let config = Arc::new(Mutex::new(config));
        let mut watcher = notify::RecommendedWatcher::new(
            EventHandler {
                config: config.clone(),
            },
            notify::Config::default(),
        )
        .unwrap();
        watcher
            .watch(&config_path, notify::RecursiveMode::Recursive)
            .unwrap();

        Self {
            watcher: Arc::new(watcher),
            path: Arc::new(config_path),
            config,
        }
    }

    pub fn read(&self) -> LockResult<MutexGuard<Config>> {
        self.config.lock()
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ConfigService {
    fn drop(&mut self) {
        // 解放した方がキレイだけどしなくてもよさそう
        // self.watcher.unwatch(&self.path).unwrap();
    }
}

struct EventHandler {
    config: Arc<Mutex<Config>>,
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
                }
            }
            notify::EventKind::Modify(kind) => {
                let notify::event::ModifyKind::Data(_) = kind else {
                    return;
                };

                // 定義ファイルが更新されたので読み込む
                for path in &e.paths {
                    *self.config.lock().unwrap() = load_config(path);
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
    println!("{}", config.image);

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
