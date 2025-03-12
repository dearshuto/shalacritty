use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use notify::{event::CreateKind, RecommendedWatcher, Watcher};

use crate::Config;

pub struct ImageCacheEx {
    watcher: RecommendedWatcher,

    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    file_watcher_receiver: tokio::sync::mpsc::Receiver<Vec<PathBuf>>,
}

impl ImageCacheEx {
    pub fn new(
        runtime: Arc<tokio::runtime::Runtime>,
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
    ) -> Self {
        let (file_watcher_sender, file_watcher_receiver) = tokio::sync::mpsc::channel(1);

        let watcher = notify::recommended_watcher(EventHandler {
            runtime,
            sender: file_watcher_sender,
        })
        .unwrap();

        Self {
            watcher,
            config_receiver,
            file_watcher_receiver,
        }
    }

    /// 画像キャッシュサービスを起動します
    pub async fn serve(mut self) {
        loop {
            tokio::select!(
                    config_opt = self.config_receiver.recv() =>
                        if let Some(config) = config_opt
            {
                        self.apply_config(&config);
            }
            else {
                        break
            },
                            Some(paths) = self.file_watcher_receiver.recv() => self.apply_files(paths.into_iter()),
                            else => break,
                        );
        }
    }

    fn apply_config(&mut self, config: &Config) {
        for path in &config.background.path {
            let path = std::path::Path::new(&path);
            self.watcher
                .watch(path, notify::RecursiveMode::NonRecursive)
                .unwrap();
        }
    }

    fn apply_files<T, I>(&mut self, _paths: I)
    where
        T: AsRef<Path>,
        I: Iterator<Item = T>,
    {
        // TODO
    }
}

struct EventHandler {
    runtime: Arc<tokio::runtime::Runtime>,
    sender: tokio::sync::mpsc::Sender<Vec<PathBuf>>,
}

impl notify::EventHandler for EventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let Ok(event) = event else {
            return;
        };

        match event.kind {
            notify::EventKind::Any => {}
            notify::EventKind::Access(_access_kind) => {}
            notify::EventKind::Create(create_kind) => {
                if create_kind == CreateKind::File {
                    self.runtime
                        .block_on(async { self.sender.send(event.paths).await.unwrap() });
                }
            }
            notify::EventKind::Modify(_modify_kind) => {
                // 本来はデータの更新のみでよいと思うが、とりあえずすべての変更をトリガーにしておく
                self.runtime
                    .block_on(async { self.sender.send(event.paths).await.unwrap() });
            }
            notify::EventKind::Remove(_remove_kind) => {}
            notify::EventKind::Other => {}
        }
    }
}
