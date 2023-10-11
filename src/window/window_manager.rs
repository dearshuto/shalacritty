use std::collections::HashMap;

use super::window::Window;

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct WindowId {
    id: uuid::Uuid,
}

impl WindowId {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
        }
    }
}

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

    #[allow(dead_code)]
    pub fn create_window(&mut self) -> WindowId {
        let window = Window {};
        let id = WindowId::new();
        self.window_table.insert(id.clone(), window);

        id
    }
}
