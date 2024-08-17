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
use tokio::{runtime::Handle, task::JoinHandle};
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
    pub fn operate_image<T>(&self, id: ImageId, mut func: T)
    where
        T: FnMut(Option<&DynamicImage>),
    {
        let Ok(binding) = self.image_cache_internal.lock() else {
            func(None);
            return;
        };

        binding.operate_image(id, func);
    }

    /// 画像データを取得します
    /// ロードが終わってなかったりファイルが壊れていると取得できないこともあります
    pub fn operate_image_and_wait<T>(&self, id: ImageId, func: T)
    where
        T: Fn(Option<&DynamicImage>),
    {
        let local = self.image_cache_internal.clone();
        // let Ok(mut binding) = self.image_cache_internal.lock() else {
        //     func(None);
        //     return;
        // };

        Handle::current().block_on(async {
            let Ok(mut binding) = local.lock() else {
                func(None);
                return;
            };
            binding.operate_image_async(id, func).await;
        });
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

    image_table: Arc<Mutex<HashMap<ImageId, DynamicImage>>>,

    generation_table: HashMap<ImageId, u64>,

    load_image_task_table: HashMap<ImageId, JoinHandle<()>>,
}

impl ImageCacheInternal {
    pub fn new() -> Self {
        Self {
            path_id_table: HashMap::default(),
            image_table: Arc::default(),
            generation_table: HashMap::default(),
            load_image_task_table: HashMap::default(),
        }
    }

    pub fn register<T>(&mut self, path: T) -> ImageId
    where
        T: AsRef<Path>,
    {
        let id = ImageId { id: Uuid::new_v4() };
        self.generation_table.insert(id, 0);

        let path_str = path.as_ref().to_str().unwrap();

        // 画像の読み込みは非同期化
        let image_table_local = self.image_table.clone();
        let path_local = path.as_ref().to_path_buf();
        let task = tokio::spawn(async move {
            // 画像を読み込んでキャッシュ
            let Some(image) = Self::load_image(path_local) else {
                return;
            };

            image_table_local.lock().unwrap().insert(id, image);
        });
        self.load_image_task_table.insert(id, task);

        self.path_id_table.insert(path_str.to_string(), id);

        id
    }

    pub fn operate_image<T>(&self, id: ImageId, func: T)
    where
        T: Fn(Option<&DynamicImage>),
    {
        let binding = self.image_table.lock().unwrap();
        let Some(image) = binding.get(&id) else {
            func(None);
            return;
        };

        func(Some(image));
    }

    pub async fn operate_image_async<T>(&mut self, id: ImageId, func: T)
    where
        T: Fn(Option<&DynamicImage>),
    {
        // タスクが実行中なら完了を待つ
        if let Some(task) = self.load_image_task_table.remove(&id) {
            let _ = task.await;
        };

        // 画像に対する処理を呼び出す
        self.operate_image(id, func);
    }

    fn update<T>(&mut self, path: T)
    where
        T: AsRef<Path>,
    {
        let path_str = path.as_ref().to_str().unwrap();

        let id = self.path_id_table.get(path_str);
        if id.is_none() {
            return;
        }

        let id = *id.unwrap();
        let id_local = id;
        let image_table_local = self.image_table.clone();
        let path_str_local = path_str.to_string();
        let task = tokio::spawn(async move {
            // ファイルが画像として不正なデータになってたらデータを破棄する
            let Some(image) = Self::load_image(path_str_local) else {
                image_table_local.lock().unwrap().remove(&id_local);
                return;
            };

            image_table_local.lock().unwrap().insert(id_local, image);
        });
        self.load_image_task_table.insert(id, task);

        *self.generation_table.get_mut(&id).unwrap() += 1;
    }

    pub fn invalidate_image<T>(&mut self, path: T)
    where
        T: AsRef<Path>,
    {
        let path_str = path.as_ref().to_str().unwrap();
        let Some(id) = self.path_id_table.get(path_str) else {
            return;
        };

        let mut binding = self.image_table.lock().unwrap();
        binding.remove(id);
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
