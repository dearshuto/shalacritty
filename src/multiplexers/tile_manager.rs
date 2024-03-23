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
    #[allow(dead_code)]
    pub fn new(mut shell_manager: TShellManager) -> (Self, TileId) {
        let id = shell_manager.spawn();
        let root_tile_id = TileId {
            internal: VirtualWindowId::default(),
        };
        let tile_id = TileId {
            internal: VirtualWindowId::default(),
        };

        let instance = Self {
            shell_manager,
            virtual_window_manager: VirtualWindowManager::new(),
            hierarchy_table: HashMap::from([(root_tile_id, vec![tile_id])]),
            root_tile_id,
            id_set: HashSet::default(),
            active_shell_id: None,
            tile_shell_table: HashMap::from([(tile_id, id)]),
        };
        (instance, tile_id)
    }

    #[allow(dead_code)]
    pub fn update(&mut self) {
        // まだ動いてるやつだけ残す
        self.id_set.retain(|id| self.shell_manager.is_running(*id));

        self.virtual_window_manager.uodate();

        // TODO: 終了している仮想ウィンドウを除外する
    }

    #[allow(dead_code)]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.virtual_window_manager.resize(width, height);
    }

    #[allow(dead_code)]
    pub fn split_horizontal(&mut self, id: TileId) -> TileId {
        let shell_id = self.shell_manager.spawn();

        let new_virtual_window_id = self
            .virtual_window_manager
            .spawn_virtual_window_with_parent(64, 64, id.internal)
            .unwrap();
        let tile_id = TileId {
            internal: new_virtual_window_id,
        };
        self.tile_shell_table.insert(tile_id, shell_id);

        tile_id
    }

    #[allow(dead_code)]
    pub fn send_input(&mut self, input: &str) {
        let Some(active_shell_id) = &self.active_shell_id else {
            return;
        };

        self.shell_manager.send_input(*active_shell_id, input);
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.id_set.is_empty()
    }
}
