use std::{collections::HashMap, sync::Arc};

use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

pub struct WindowManager {
    window_table: HashMap<WindowId, Arc<Window>>,
}

impl WindowManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            window_table: Default::default(),
        }
    }

    pub async fn create_window<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) -> WindowId {
        let window = WindowBuilder::new()
            .with_transparent(true)
            .with_min_inner_size(PhysicalSize::new(300, 300))
            .build(event_loop)
            .unwrap();
        let id = window.id();
        self.window_table.insert(id, Arc::new(window));
        id
    }

    pub fn try_get_window(&self, id: WindowId) -> Option<Arc<Window>> {
        let Some(window) = self.window_table.get(&id) else {
            return None;
        };

        Some(window.clone())
    }
}
