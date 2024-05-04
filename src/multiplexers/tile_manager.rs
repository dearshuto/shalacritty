use std::collections::{HashMap, HashSet};

use super::{
    detail::{VirtualWindowId, VirtualWindowManager},
    IShellManager,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileId {
    internal: VirtualWindowId,
}

#[allow(dead_code)]
pub struct TileManager<TShellManager: IShellManager> {
    shell_manager: TShellManager,

    // タイリングに使用する仮想ウィンドウ
    virtual_window_manager: VirtualWindowManager,

    tile_shell_table: HashMap<TileId, TShellManager::Id>,

    // 親ウィンドウ -> 子ウィンドウ
    hierarchy_table: HashMap<TileId, Vec<TileId>>,

    root_tile_id: TileId,

    id_set: HashSet<TShellManager::Id>,

    active_shell_id: Option<TShellManager::Id>,
}

impl<TShellManager: IShellManager> TileManager<TShellManager> {
    pub fn new(mut shell_manager: TShellManager) -> (Self, TileId) {
        let shell_id = shell_manager.spawn();
        let root_tile_id = TileId {
            internal: VirtualWindowId::default(),
        };

        let mut virtual_window_manager = VirtualWindowManager::new();
        let virtual_window_id = virtual_window_manager.spawn_virtual_window(1280, 960);

        let tile_id = TileId {
            internal: virtual_window_id,
        };

        virtual_window_manager.update();

        let instance = Self {
            shell_manager,
            virtual_window_manager,
            hierarchy_table: HashMap::from([(root_tile_id, vec![tile_id])]),
            root_tile_id,
            id_set: HashSet::from([shell_id]),
            active_shell_id: Some(shell_id),
            tile_shell_table: HashMap::from([(tile_id, shell_id)]),
        };
        (instance, tile_id)
    }

    pub fn update(&mut self) {
        self.shell_manager.update();

        // まだ動いてるやつだけ残す
        self.id_set.retain(|id| self.shell_manager.is_running(*id));

        self.virtual_window_manager.update();

        // TODO: 終了している仮想ウィンドウを除外する
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // 描画領域を計算
        self.virtual_window_manager.resize(width, height);
        self.virtual_window_manager.update();

        // 描画領域をシェルに反映
        for (tile_id, shell_id) in &self.tile_shell_table {
            let virtual_window_id = tile_id.internal;

            let Some((width, height)) = self
                .virtual_window_manager
                .try_get_actual_size(virtual_window_id)
            else {
                continue;
            };

            self.shell_manager
                .resize(*shell_id, width as i32, height as i32);
        }
    }

    #[allow(dead_code)]
    pub fn split_horizontal(&mut self, id: TileId) -> TileId {
        let virtual_window_id = id.internal;

        // 仮想ウィンドウを分割
        // 更新処理はここじゃなくてもよいかも？
        let new_virtual_window_id = self
            .virtual_window_manager
            .split_horizontal(virtual_window_id);
        self.virtual_window_manager.update();

        // 新規に追加した仮想ウィンドウに割り当てるシェルを起動
        let shell_id = self.shell_manager.spawn();

        // 分割した下側
        // TileId を新たに発行して分割したウィンドウに紐付ける
        let tile_id = TileId {
            internal: new_virtual_window_id,
        };
        self.tile_shell_table.insert(tile_id, shell_id);

        tile_id
    }

    pub fn send_input(&mut self, input: &str) {
        let Some(active_shell_id) = &self.active_shell_id else {
            return;
        };

        self.shell_manager.send_input(*active_shell_id, input);
    }

    pub fn enumerate_content(
        &self,
        id: TileId,
    ) -> impl Iterator<Item = TShellManager::Content> + '_ {
        let child_virtual_window_ids = self.virtual_window_manager.find_children(id.internal);
        let mut ids = vec![id.internal];
        ids.extend(child_virtual_window_ids);

        let mut contents = Vec::default();
        for virtual_window_id in ids {
            let tile_id = TileId {
                internal: virtual_window_id,
            };
            let shell_id = self.tile_shell_table.get(&tile_id).unwrap();
            let content = self.shell_manager.enumerate_content(*shell_id);
            contents.extend(content);
        }
        contents.into_iter()
    }

    pub fn get_cursor_position(&self, id: TileId) -> (u32, u32) {
        let shell_id = self.tile_shell_table.get(&id).unwrap();
        self.shell_manager.get_cursor_position(*shell_id)
    }

    pub fn is_empty(&self) -> bool {
        self.id_set.is_empty()
    }

    pub fn consume_dirty(&mut self, id: TileId) -> Option<bool> {
        let shell_id = self.tile_shell_table.get(&id).unwrap();
        self.shell_manager.consume_dirty(*shell_id)
    }
}
