use std::{path::PathBuf, str::FromStr};

use notify::Watcher;

pub struct Config {}

pub struct ConfigService {
    watcher: Box<dyn notify::Watcher>,
    path: PathBuf,
}

impl ConfigService {
    pub fn new() -> Self {
        // コンフィグ置き場。なければ作る。
        let config_path = create_config_directory();
        if !config_path.exists() {
            std::fs::DirBuilder::new().create(&config_path).unwrap();
        }

        let mut watcher =
            notify::RecommendedWatcher::new(EventHandler {}, notify::Config::default()).unwrap();
        watcher
            .watch(&config_path, notify::RecursiveMode::Recursive)
            .unwrap();

        Self {
            watcher: Box::new(watcher),
            path: config_path,
        }
    }
}

impl Drop for ConfigService {
    fn drop(&mut self) {
        self.watcher.unwatch(&self.path).unwrap();
    }
}

struct EventHandler;
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
                    println!("{:?}: {:?}", e.kind, path);
                    let _file = std::fs::File::open(&path).unwrap();
                }
            }
            // notify::EventKind::Remove(_) => todo!(),
            // notify::EventKind::Other => todo!(),
            _ => {}
        }
    }
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
