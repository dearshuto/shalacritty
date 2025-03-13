use std::path::Path;

use notify::Watcher;

use crate::{config::util, Config};

pub struct EventHandler {
    // 初期化以降もインスタンスとしては保持しておきたいので警告を抑制
    #[allow(unused)]
    watcher: notify::RecommendedWatcher,
}

impl EventHandler {
    pub fn new<TPath>(config_dir: TPath) -> (Self, tokio::sync::watch::Receiver<Config>)
    where
        TPath: AsRef<Path>,
    {
        // コンフィグファイルのパス
        let config_path = {
            let mut config_path = config_dir.as_ref().to_path_buf();
            config_path.push("config.toml");
            config_path
        };

        // ロードしたコンフィグを初期値としてチャンネルを作成
        let config = crate::config::util::load_config(&config_path);
        let (sender, receiver) = tokio::sync::watch::channel(config);

        // config.toml の監視を開始
        let adapter = EventHandlerAdapter::new(sender);
        let mut watcher =
            notify::RecommendedWatcher::new(adapter, notify::Config::default()).unwrap();
        watcher
            .watch(&config_path.as_ref(), notify::RecursiveMode::Recursive)
            .unwrap();

        (Self { watcher }, receiver)
    }
}

// 本来は private でよいが、後方互換のために後悔している
pub struct EventHandlerAdapter {
    sender: tokio::sync::watch::Sender<Config>,
}

impl EventHandlerAdapter {
    pub fn new(sender: tokio::sync::watch::Sender<Config>) -> Self {
        Self { sender }
    }
}

impl notify::EventHandler for EventHandlerAdapter {
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
                    let _file = std::fs::File::open(path).unwrap();
                    self.sender.send(Config::default()).unwrap();
                }
            }
            notify::EventKind::Modify(kind) => {
                let notify::event::ModifyKind::Data(_) = kind else {
                    return;
                };

                println!("adfs");

                // 定義ファイルが更新されたので読み込む
                for path in &e.paths {
                    let config = util::load_config(path);
                    self.sender.send(config).unwrap();
                }
            }
            // notify::EventKind::Remove(_) => todo!(),
            // notify::EventKind::Other => todo!(),
            _ => {}
        }
    }
}
