use std::collections::HashMap;

use winit::{
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

pub struct WindowManager {
    window_table: HashMap<WindowId, Window>,
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
            .build(event_loop)
            .unwrap();
        let id = window.id();
        self.window_table.insert(id, window);
        id
    }

    pub fn try_get_window(&self, id: WindowId) -> Option<&Window> {
        self.window_table.get(&id)
    }
}
