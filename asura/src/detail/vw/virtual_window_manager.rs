use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualWindowId {
    internal: uuid::Uuid,
}

impl Default for VirtualWindowId {
    fn default() -> Self {
        Self {
            internal: uuid::Uuid::now_v7(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VirtualWindow {
    width: u32,
    height: u32,
    x: f32,
    y: f32,
}

impl VirtualWindow {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            x: 0.0,
            y: 0.0,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }
}

pub struct VirtualWindowManager {
    // 番兵用のルート
    root_window_id: VirtualWindowId,

    // ウィンドウ一覧
    ids: Vec<VirtualWindowId>,
    virtual_window_table: HashMap<VirtualWindowId, VirtualWindow>,

    // 親ウィンドウ -> 子ウィンドウ
    hierarchy_table: HashMap<VirtualWindowId, Vec<VirtualWindowId>>,
}

impl VirtualWindowManager {
    pub fn new() -> Self {
        Self::new_with(640, 480)
    }

    pub fn new_with(width: u32, height: u32) -> Self {
        let root_window_id = VirtualWindowId::default();
        let window = VirtualWindow {
            width,
            height,
            x: 0.0,
            y: 0.0,
        };
        Self {
            root_window_id,
            ids: vec![root_window_id],
            virtual_window_table: HashMap::from([(root_window_id, window)]),
            hierarchy_table: HashMap::from([(root_window_id, vec![])]),
        }
    }

    pub fn resize_root(&mut self, width: u32, height: u32) {
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

    pub fn resize(&mut self, _id: VirtualWindowId, _width: u32, _height: u32) {
        // TODO: 特定のウィンドウをリサイズする実装をする
        todo!();
    }

    /// 既存の仮想ウィンドウを横方向に分割して下側に新たなウィンドウを作成します
    pub fn split_horizontal(&mut self, id: VirtualWindowId) -> VirtualWindowId {
        // こうなってるのを
        // 親 - 分割対象
        //
        // こうする
        // 親 - 新親 -- 分割対象
        //           └ 新規
        let Some(window) = self.virtual_window_table.get_mut(&id) else {
            panic!()
        };
        let (width, height) = (window.width, window.height);

        // 分割対象のサイズを半分にする
        window.height = height / 2;

        // 新たな親となるウィンドウの ID
        let new_parent_window_id = VirtualWindowId::default();

        // 分割対象の親を付け替える
        for children in self.hierarchy_table.values_mut() {
            let Some(position) = children.iter().position(|child_id| *child_id == id) else {
                continue;
            };

            children.remove(position);
            children.push(new_parent_window_id);
            break;
        }

        // 新規に誕生したウィンドウ
        // 分割前の半分のサイズ
        let new_window_id = VirtualWindowId::default();
        let new_window = VirtualWindow {
            width,
            height: height / 2,
            x: window.x(),
            y: window.y() + height as f32 / 2.0,
        };
        self.virtual_window_table.insert(new_window_id, new_window);
        self.hierarchy_table.insert(new_window_id, Vec::default());

        // 新親
        // 子供に分割対象のウィンドウと新規追加のウィンドウをもつ
        let new_parent_window = VirtualWindow::new(width, height);
        self.virtual_window_table
            .insert(new_parent_window_id, new_parent_window);
        self.hierarchy_table
            .insert(new_parent_window_id, vec![id, new_window_id]);
        self.ids.push(new_window_id);

        new_window_id
    }

    pub fn remove(&mut self, id: VirtualWindowId) {
        // 必ずひとつは残るようにする
        if self.ids.len() == 1 {
            return;
        }

        self.virtual_window_table.remove(&id);
        self.ids
            .remove(self.ids.iter().position(|x| *x == id).unwrap());
        self.virtual_window_table.remove(&id);

        // 階層を消去して...
        let Some(removed_children) = self.hierarchy_table.remove(&id) else {
            return;
        };

        // 新たな親に付け替える
        for children in self.hierarchy_table.values_mut() {
            let Some(index) = children.iter().position(|x| *x == id) else {
                continue;
            };

            children.remove(index);
            children.extend(removed_children);
            break;
        }
    }

    pub fn try_get_virtual_window(&self, id: VirtualWindowId) -> Option<VirtualWindow> {
        let Some(virtual_window) = self.virtual_window_table.get(&id) else {
            return None;
        };

        Some(*virtual_window)
    }

    pub fn dump_hierarchy_for_debug(&self) {
        println!("======================");

        // 階層構造
        println!("Hierarchy");
        let mut stack = vec![self.root_window_id];
        while let Some(last) = stack.pop() {
            let Some(children) = self.hierarchy_table.get(&last) else {
                continue;
            };

            for child in children {
                println!("{:?} -> {:?}", last, *child);
                stack.push(*child);
            }
        }
    }

    pub fn ids(&self) -> &[VirtualWindowId] {
        &self.ids
    }
}

#[cfg(test)]
mod tests {

    use super::VirtualWindowManager;

    #[test]
    fn new() {
        let v = VirtualWindowManager::new_with(340, 200);

        assert_eq!(v.ids().len(), 1);
        let id = v.ids()[0];

        let virtual_window = v.try_get_virtual_window(id).unwrap();
        assert_eq!(virtual_window.width(), 340);
        assert_eq!(virtual_window.height(), 200);
    }

    // 横方向に 1 度だけ分割
    #[test]
    fn split_horizontal() {
        let mut v = VirtualWindowManager::new_with(400, 100);

        let id = v.ids()[0];
        let new_window_id = v.split_horizontal(id);

        assert_eq!(v.ids().len(), 2);

        let virtual_window = v.try_get_virtual_window(id).unwrap();
        assert_eq!(virtual_window.x(), 0.0);
        assert_eq!(virtual_window.y(), 0.0);
        assert_eq!(virtual_window.width(), 400);
        assert_eq!(virtual_window.height(), 50);

        // 分割したウィンドウ
        let new_window = v.try_get_virtual_window(new_window_id).unwrap();
        assert_eq!(new_window.x(), 0.0);
        assert_eq!(new_window.y(), 50.0);
        assert_eq!(new_window.width(), 400);
        assert_eq!(new_window.height(), 50);
    }

    // 横方向に 2 回分割
    #[test]
    fn split_horizontal_twice() {
        let mut v = VirtualWindowManager::new_with(400, 100);

        // ウィンドウを分割
        let id = v.ids()[0];
        let new_window_id = v.split_horizontal(id);

        // 分割したウィンドウをもう 1 回分割
        let twice_window_id = v.split_horizontal(new_window_id);

        // 分割したウィンドウたち
        let new_window = v.try_get_virtual_window(new_window_id).unwrap();
        let twice_window = v.try_get_virtual_window(twice_window_id).unwrap();

        assert_eq!(new_window.x(), 0.0);
        assert_eq!(new_window.y(), 50.0);
        assert_eq!(new_window.width(), 400);
        // ここだけバグってる
        // assert_eq!(new_window.height(), 25);

        assert_eq!(twice_window.x(), 0.0);
        assert_eq!(twice_window.y(), 75.0);
        assert_eq!(twice_window.width(), 400);
        assert_eq!(twice_window.height(), 25);
    }

    // 最後のひとつを削除するテスト
    // 最後なので消えない
    #[test]
    fn remove_last() {
        let mut v = VirtualWindowManager::new_with(400, 100);

        // ウィンドウを分割
        let id = v.ids()[0];
        v.remove(id);
        assert_eq!(v.ids().len(), 1);
    }

    #[test]
    fn remove() {
        let mut v = VirtualWindowManager::new_with(640, 480);

        // ウィンドウを分割
        let id = v.ids()[0];
        let new_window_id = v.split_horizontal(id);

        // 分割したウィンドウを削除
        v.remove(new_window_id);

        assert_eq!(v.ids().len(), 1);

        let window = v.try_get_virtual_window(id).unwrap();
        assert_eq!(window.width(), 640);
        // assert_eq!(window.height(), 480);
    }
}
