use std::{
    fs::File,
    path::{Path, PathBuf},
};

use image::codecs::{jpeg::JpegDecoder, png::PngDecoder};
use notify::{event::CreateKind, RecommendedWatcher, Watcher};
use tracing::instrument;

use crate::Config;

pub struct ImageLoadedEventArgs {
    pub image: image::DynamicImage,
}

pub struct ImageCacheEx {
    watcher: RecommendedWatcher,

    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    file_watcher_receiver: tokio::sync::mpsc::Receiver<Vec<PathBuf>>,

    image_sender: tokio::sync::mpsc::Sender<ImageLoadedEventArgs>,
}

impl ImageCacheEx {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
    ) -> (Self, tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>) {
        let (file_watcher_sender, file_watcher_receiver) = tokio::sync::mpsc::channel(1);

        let watcher = notify::recommended_watcher(EventHandler {
            sender: file_watcher_sender,
        })
        .unwrap();

        let (image_sender, image_receiver) = tokio::sync::mpsc::channel(1);

        (
            Self {
                watcher,
                config_receiver,
                file_watcher_receiver,
                image_sender,
            },
            image_receiver,
        )
    }

    /// 画像キャッシュサービスを起動します
    #[instrument]
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
                            Some(paths) = self.file_watcher_receiver.recv() => self.apply_files(paths.into_iter()).await,
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

    async fn apply_files<T, I>(&mut self, paths: I)
    where
        T: AsRef<Path>,
        I: Iterator<Item = T>,
    {
        for path in paths {
            let Some(extension) = path.as_ref().extension() else {
                continue;
            };

            let mut reader = File::open(path.as_ref()).unwrap();

            let image = if extension == "png" || extension == "PNG" {
                let decoder = PngDecoder::new(&mut reader).unwrap();
                image::DynamicImage::from_decoder(decoder).unwrap()
            } else if extension == "jpg" || extension == "JPG" {
                let decoder = JpegDecoder::new(&mut reader).unwrap();
                image::DynamicImage::from_decoder(decoder).unwrap()
            } else {
                panic!()
            };

            let args = ImageLoadedEventArgs { image };
            self.image_sender.send(args).await.unwrap();
        }
    }
}

impl std::fmt::Debug for ImageCacheEx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ImageCacheEx")?;
        std::fmt::Result::Ok(())
    }
}

struct EventHandler {
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
                    self.sender.blocking_send(event.paths).unwrap();
                }
            }
            notify::EventKind::Modify(_modify_kind) => {
                // 本来はデータの更新のみでよいと思うが、とりあえずすべての変更をトリガーにしておく
                self.sender.blocking_send(event.paths).unwrap();
            }
            notify::EventKind::Remove(_remove_kind) => {}
            notify::EventKind::Other => {}
        }
    }
}
