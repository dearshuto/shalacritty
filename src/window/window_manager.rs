use std::{collections::HashMap, sync::Arc};

use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

pub struct WindowManager {
    // 生成順にソート
    ids: Vec<WindowId>,

    window_table: HashMap<WindowId, Arc<Window>>,
}

impl WindowManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            ids: Vec::default(),
            window_table: Default::default(),
        }
    }

    pub async fn create_window<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) -> WindowId {
        // カラーターゲットの最大値を 2048x2048 に設定しているのでウィンドウサイズもそれを超えないようにしている
        let window = WindowBuilder::new()
            .with_transparent(true)
            .with_min_inner_size(PhysicalSize::new(300, 300))
            .with_max_inner_size(PhysicalSize::new(2048, 2048))
            .build(event_loop)
            .unwrap();
        let id = window.id();
        self.ids.push(id);
        self.window_table.insert(id, Arc::new(window));
        id
    }

    pub fn try_get_window(&self, id: WindowId) -> Option<Arc<Window>> {
        let Some(window) = self.window_table.get(&id) else {
            return None;
        };

        Some(window.clone())
    }

    pub fn ids(&self) -> &[WindowId] {
        &self.ids
    }
}
