use std::{
    collections::HashMap,
    fs::File,
    path::Path,
    sync::{Arc, Mutex},
};

use image::{
    codecs::{jpeg::JpegDecoder, png::PngDecoder},
    DynamicImage,
};
use notify::{
    event::{CreateKind, RemoveKind},
    RecommendedWatcher, RecursiveMode, Watcher,
};
use uuid::Uuid;

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub struct ImageId {
    id: Uuid,
}

pub struct ImageCache {
    watcher: RecommendedWatcher,

    image_cache_internal: Arc<Mutex<ImageCacheInternal>>,
}

impl ImageCache {
    pub fn new() -> Self {
        let image_cache_internal = Arc::new(Mutex::new(ImageCacheInternal::new()));
        let watcher: RecommendedWatcher =
            notify::recommended_watcher(EventHanlder::new(image_cache_internal.clone())).unwrap();

        ImageCache {
            watcher,
            image_cache_internal,
        }
    }

    pub fn register<TPath: AsRef<Path>>(&mut self, path: TPath) -> Option<ImageId> {
        // ファイルを監視
        let Ok(()) = self
            .watcher
            .watch(path.as_ref(), RecursiveMode::NonRecursive)
        else {
            return None;
        };

        Some(self.image_cache_internal.lock().unwrap().register(path))
    }

    #[allow(dead_code)]
    pub fn unregister(&mut self, _id: ImageId) {}

    /// 画像データを取得します
    /// ロードが終わってなかったりファイルが壊れていると取得できないこともあります
    pub fn operate_image<T>(&self, id: ImageId, func: T)
    where
        T: Fn(Option<&DynamicImage>),
    {
        let Ok(binding) = self.image_cache_internal.lock() else {
            func(None);
            return;
        };

        let Some(image) = binding.image_table.get(&id) else {
            func(None);
            return;
        };

        func(Some(image));
    }

    pub fn get_generation(&self, id: ImageId) -> u64 {
        self.image_cache_internal.lock().unwrap().get_generation(id)
    }
}

struct EventHanlder {
    image_cache_internal: Arc<Mutex<ImageCacheInternal>>,
}

impl EventHanlder {
    pub fn new(image_cache_internal: Arc<Mutex<ImageCacheInternal>>) -> Self {
        Self {
            image_cache_internal,
        }
    }

    fn on_file_created<TPath>(&mut self, path: TPath)
    where
        TPath: AsRef<Path>,
    {
        self.image_cache_internal.lock().unwrap().register(path);
    }

    fn on_file_updated<TPath>(&mut self, path: TPath)
    where
        TPath: AsRef<Path>,
    {
        self.image_cache_internal.lock().unwrap().update(path);
    }

    fn on_file_removed<TPath>(&mut self, path: TPath)
    where
        TPath: AsRef<Path>,
    {
        self.image_cache_internal
            .lock()
            .unwrap()
            .invalidate_image(path);
    }
}

impl notify::EventHandler for EventHanlder {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let Ok(e) = event else {
            return;
        };

        match e.kind {
            notify::EventKind::Any => {}
            notify::EventKind::Access(_) => {}
            notify::EventKind::Create(kind) => {
                if kind == CreateKind::File {
                    for path in e.paths {
                        self.on_file_created(path)
                    }
                }
            }
            notify::EventKind::Modify(_) => {
                for path in e.paths {
                    self.on_file_updated(path);
                }
            }
            notify::EventKind::Remove(kind) => {
                if kind == RemoveKind::File {
                    for path in e.paths {
                        self.on_file_removed(path);
                    }
                }
            }
            notify::EventKind::Other => {}
        }
    }
}

struct ImageCacheInternal {
    path_id_table: HashMap<String, ImageId>,

    image_table: HashMap<ImageId, DynamicImage>,

    generation_table: HashMap<ImageId, u64>,
}

impl ImageCacheInternal {
    pub fn new() -> Self {
        Self {
            path_id_table: HashMap::default(),
            image_table: HashMap::default(),
            generation_table: HashMap::default(),
        }
    }

    pub fn register<T>(&mut self, path: T) -> ImageId
    where
        T: AsRef<Path>,
    {
        let id = ImageId { id: Uuid::new_v4() };
        self.generation_table.insert(id, 0);

        let path_str = path.as_ref().to_str().unwrap();

        // 画像を読み込んでキャッシュ
        let Some(image) = Self::load_image(path.as_ref()) else {
            return id;
        };

        self.path_id_table.insert(path_str.to_string(), id);
        self.image_table.insert(id, image);
        id
    }

    pub fn update<T>(&mut self, path: T)
    where
        T: AsRef<Path>,
    {
        let path_str = path.as_ref().to_str().unwrap();

        let Some(id) = self.path_id_table.get(path_str) else {
            return;
        };

        // ファイルが画像として不正なデータになってたらデータを破棄する
        let Some(image) = Self::load_image(path) else {
            self.image_table.remove(id);
            return;
        };

        self.image_table.insert(*id, image);
        *self.generation_table.get_mut(id).unwrap() += 1;
    }

    pub fn invalidate_image<T>(&mut self, path: T)
    where
        T: AsRef<Path>,
    {
        let path_str = path.as_ref().to_str().unwrap();
        let Some(id) = self.path_id_table.get(path_str) else {
            return;
        };

        self.image_table.remove(id);
        *self.generation_table.get_mut(id).unwrap() += 1;
    }

    pub fn get_generation(&self, id: ImageId) -> u64 {
        *self.generation_table.get(&id).unwrap()
    }

    fn load_image<T>(path: T) -> Option<DynamicImage>
    where
        T: AsRef<Path>,
    {
        // 画像を読み込んでキャッシュ
        let mut reader = File::open(path.as_ref()).unwrap();
        let extension = path.as_ref().extension()?.to_str()?;

        match extension {
            "png" => {
                let decoder = PngDecoder::new(&mut reader).unwrap();
                Some(DynamicImage::from_decoder(decoder).unwrap())
            }
            "PNG" => {
                let decoder = PngDecoder::new(&mut reader).unwrap();
                Some(DynamicImage::from_decoder(decoder).unwrap())
            }
            "jpg" => {
                let decoder = JpegDecoder::new(&mut reader).unwrap();
                Some(DynamicImage::from_decoder(decoder).unwrap())
            }
            "JPG" => {
                let decoder = JpegDecoder::new(&mut reader).unwrap();
                Some(DynamicImage::from_decoder(decoder).unwrap())
            }
            _ => None,
        }
    }
}
