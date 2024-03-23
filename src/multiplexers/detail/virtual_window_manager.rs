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

    // 各ウィンドウの実際のサイズ
    actual_size_table: HashMap<VirtualWindowId, (u32, u32)>,
}

impl VirtualWindowManager {
    pub fn new() -> Self {
        let root_window_id = VirtualWindowId::default();
        let mut hierarchy_table = HashMap::default();
        hierarchy_table.insert(root_window_id, Vec::default());

        let mut actual_size_table = HashMap::new();
        actual_size_table.insert(root_window_id, (0, 0));

        Self {
            root_window_id,
            virtual_window_table: HashMap::new(),
            hierarchy_table,
            actual_size_table,
        }
    }

    pub fn uodate(&mut self) {
        let Some(parent_ids) = self.hierarchy_table.get(&self.root_window_id) else {
            panic!();
        };

        for parent_window_id in parent_ids {
            let Some(window) = self.virtual_window_table.get(parent_window_id) else {
                continue;
            };

            let Some((width, height)) = self.actual_size_table.get_mut(parent_window_id) else {
                continue;
            };

            *width = window.width;
            *height = window.height;
        }

        for parent_window_id in parent_ids {
            Self::update_recursive(
                &mut self.actual_size_table,
                *parent_window_id,
                &self.virtual_window_table,
                &self.hierarchy_table,
            );
        }
    }

    fn update_recursive(
        actual_size_table: &mut HashMap<VirtualWindowId, (u32, u32)>,
        id: VirtualWindowId,
        virtual_window_table: &HashMap<VirtualWindowId, VirtualWindow>,
        hierarchy_table: &HashMap<VirtualWindowId, Vec<VirtualWindowId>>,
    ) {
        let Some(children) = hierarchy_table.get(&id) else {
            panic!();
        };

        let Some((_parent_width, parent_height)) = actual_size_table.get(&id) else {
            return;
        };

        let mut h = *parent_height;
        for child_id in children {
            let Some((width, height)) = actual_size_table.get_mut(child_id) else {
                continue;
            };

            let Some(child_window) = virtual_window_table.get(child_id) else {
                continue;
            };

            let is_last_item = children.last().unwrap() == child_id;
            let new_actual_height = if is_last_item {
                h
            } else {
                child_window.height.min(h)
            };

            *width = child_window.width;
            *height = new_actual_height;
            h -= new_actual_height;
        }

        for child_id in children {
            Self::update_recursive(
                actual_size_table,
                *child_id,
                virtual_window_table,
                hierarchy_table,
            );
        }
    }

    pub fn spawn_virtual_window(&mut self, width: u32, height: u32) -> VirtualWindowId {
        // デフォルトはルートを親としてウィンドウを作成
        self.spawn_virtual_window_with_parent(width, height, self.root_window_id)
            .unwrap()
    }

    pub fn spawn_virtual_window_with_parent(
        &mut self,
        width: u32,
        height: u32,
        parent_id: VirtualWindowId,
    ) -> Option<VirtualWindowId> {
        // 存在しない親を指定してないかチェック
        let Some(children) = self.hierarchy_table.get_mut(&parent_id) else {
            return None;
        };

        // ウィンドウのインスタンスを作成
        let id = VirtualWindowId::default();
        let virtual_window = VirtualWindow::new(width, height);
        self.virtual_window_table.insert(id, virtual_window);
        children.push(id);

        // 階層構造用のデータを追加
        self.hierarchy_table.insert(id, Vec::default());

        // サイズ計算用のデータを追加
        self.actual_size_table.insert(id, (0, 0));

        Some(id)
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

    #[allow(dead_code)]
    pub fn try_get_actual_size(&self, id: VirtualWindowId) -> Option<(u32, u32)> {
        let Some((width, height)) = self.actual_size_table.get(&id) else {
            return None;
        };

        Some((*width, *height))
    }
}

#[cfg(test)]
mod tests {

    use super::VirtualWindowManager;

    // 一番親のウィンドウのサイズ
    #[test]
    fn parents() {
        let mut manager = VirtualWindowManager::new();
        let id = manager.spawn_virtual_window(640, 480);
        manager.uodate();

        let (width, height) = manager.try_get_actual_size(id).unwrap();
        assert_eq!(width, 640);
        assert_eq!(height, 480);
    }

    // 子供が一人だけなら親子で同じサイズ
    #[test]
    fn single_child() {
        let mut manager = VirtualWindowManager::new();
        let id = manager.spawn_virtual_window(640, 480);
        let child_id = manager
            .spawn_virtual_window_with_parent(640, 480, id)
            .unwrap();
        manager.uodate();

        let (width, height) = manager.try_get_actual_size(id).unwrap();
        assert_eq!(width, 640);
        assert_eq!(height, 480);

        let (width, height) = manager.try_get_actual_size(child_id).unwrap();
        assert_eq!(width, 640);
        assert_eq!(height, 480);
    }

    // 子供が一人だけなら親子で同じサイズ
    #[test]
    fn children() {
        let mut manager = VirtualWindowManager::new();
        let id = manager.spawn_virtual_window(640, 480);
        let child_id0 = manager
            .spawn_virtual_window_with_parent(640, 240, id)
            .unwrap();
        let child_id1 = manager
            .spawn_virtual_window_with_parent(640, 240, id)
            .unwrap();

        manager.uodate();

        let (width, height) = manager.try_get_actual_size(id).unwrap();
        assert_eq!(width, 640);
        assert_eq!(height, 480);

        let (width, height) = manager.try_get_actual_size(child_id0).unwrap();
        assert_eq!(width, 640);
        assert_eq!(height, 240);

        let (width, height) = manager.try_get_actual_size(child_id1).unwrap();
        assert_eq!(width, 640);
        assert_eq!(height, 240);
    }
}
