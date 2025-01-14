use std::{collections::HashMap, sync::Arc};

use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

pub struct WindowManager {
    // 生成順にソート
    ids: Vec<WindowId>,

    window_table: HashMap<WindowId, Arc<Window>>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            ids: Vec::default(),
            window_table: Default::default(),
        }
    }

    pub async fn create_window(&mut self, event_loop: &ActiveEventLoop) -> WindowId {
        // カラーターゲットの最大値を 2048x2048 に設定しているのでウィンドウサイズもそれを超えないようにしている
        let window_attributes = WindowAttributes::default()
            .with_transparent(true)
            .with_min_inner_size(PhysicalSize::new(300, 300))
            .with_max_inner_size(PhysicalSize::new(4096, 4096));
        let window = event_loop.create_window(window_attributes).unwrap();
        window.set_ime_allowed(true);

        let id = window.id();
        self.ids.push(id);
        self.window_table.insert(id, Arc::new(window));
        id
    }

    pub fn try_get_window(&self, id: WindowId) -> Option<Arc<Window>> {
        let window = self.window_table.get(&id)?;

        Some(window.clone())
    }

    pub fn ids(&self) -> &[WindowId] {
        &self.ids
    }
}
