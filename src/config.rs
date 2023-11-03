use std::{
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

use notify::Watcher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub background: [f32; 4],
}

pub struct ConfigService {
    watcher: Box<dyn notify::Watcher>,
    path: PathBuf,
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
            watcher: Box::new(watcher),
            path: config_path,
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
        self.watcher.unwatch(&self.path).unwrap();
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
                    let _file = std::fs::File::open(&path).unwrap();
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
    toml::from_str(&str).unwrap()
}

fn create_config_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    let home_directory = std::env::var("FOLDERID_RoamingAppData").unwrap();

    #[cfg(target_os = "macos")]
    let home_directory = std::env::var("HOME").unwrap();

    #[cfg(target_os = "linux")]
    let home_directory = std::env::var("HOME").unwrap();

    let mut config_directory_path = PathBuf::from_str(&home_directory).unwrap();
    config_directory_path.push(".config");
    config_directory_path.push("shalacritty");

    return config_directory_path;
}
