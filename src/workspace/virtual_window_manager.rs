use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualWindowId {
    internal: uuid::Uuid,
}

impl Default for VirtualWindowId {
    fn default() -> Self {
        Self {
            internal: uuid::Uuid::new_v4(),
        }
    }
}

pub struct VirtualWindow {
    #[allow(dead_code)]
    pub width: u32,
    #[allow(dead_code)]
    pub height: u32,
}

impl VirtualWindow {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

pub struct VirtualWindowManager {
    // 番兵用のルート
    root_window_id: VirtualWindowId,

    // ウィンドウ一覧
    virtual_window_table: HashMap<VirtualWindowId, VirtualWindow>,

    // 親ウィンドウ -> 子ウィンドウ
    hierarchy_table: HashMap<VirtualWindowId, Vec<VirtualWindowId>>,
}

impl VirtualWindowManager {
    pub fn new() -> Self {
        Self {
            root_window_id: VirtualWindowId::default(),
            virtual_window_table: HashMap::new(),
            hierarchy_table: HashMap::default(),
        }
    }

    pub fn spawn_virtual_window(&mut self, width: u32, height: u32) -> VirtualWindowId {
        let id = VirtualWindowId::default();
        let virtual_window = VirtualWindow::new(width, height);
        self.virtual_window_table.insert(id, virtual_window);
        id
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let Some(root_windows) = self.hierarchy_table.get(&self.root_window_id) else {
            return;
        };
        for id in root_windows {
            let Some(window) = self.virtual_window_table.get_mut(id) else {
                continue;
            };

            window.width = width;
            window.height = height;
        }
    }

    pub fn try_get_window(&self, id: VirtualWindowId) -> Option<&VirtualWindow> {
        self.virtual_window_table.get(&id)
    }
}
