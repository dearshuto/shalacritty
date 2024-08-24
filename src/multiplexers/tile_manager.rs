use std::collections::{HashMap, HashSet};

use crate::multiplexers::shell_manager::IPosition;
use crate::multiplexers::IContent;

use super::{
    detail::{TabId, VirtualWindowId, VirtualWindowManager},
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

    active_tab_id: TabId,

    // アクティブなタブで操作対象になってるシェル
    active_shell_table: HashMap<TabId, Vec<TShellManager::Id>>,
}

impl<TShellManager: IShellManager> TileManager<TShellManager> {
    pub fn new(mut shell_manager: TShellManager) -> (Self, TileId) {
        let shell_id = shell_manager.spawn();
        let root_tile_id = TileId {
            internal: VirtualWindowId::default(),
        };

        let mut virtual_window_manager = VirtualWindowManager::new();
        let tab_id = virtual_window_manager.spawn_virtual_window(1280, 960);
        let virtual_window_id = *virtual_window_manager
            .find_children(tab_id)
            .first()
            .unwrap();

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
            active_tab_id: tab_id,
            active_shell_table: HashMap::from([(tab_id, vec![shell_id])]),
            tile_shell_table: HashMap::from([(tile_id, shell_id)]),
        };
        (instance, tile_id)
    }

    pub fn update(&mut self) {
        self.shell_manager.update();

        // 終了してるシェル削除しつつ確保しておく
        let mut killed_shell_ids = Vec::default();
        self.id_set.retain(|id| {
            if self.shell_manager.is_running(*id) {
                return true;
            }

            killed_shell_ids.push(*id);
            false
        });

        // 終了したシェルの仮想ウィンドウを削除
        for killed_shell_id in killed_shell_ids {
            let Some((tile_id, _shell_id)) = self
                .tile_shell_table
                .iter()
                .find(|(_tile_id, shell_id)| **shell_id == killed_shell_id)
            else {
                continue;
            };

            let virtual_window_id = tile_id.internal;
            self.virtual_window_manager.remove(virtual_window_id);
            break;
        }

        self.virtual_window_manager.update();

        if !self
            .virtual_window_manager
            .get_tab_ids()
            .contains(&self.active_tab_id)
            && !self.virtual_window_manager.get_tab_ids().is_empty()
        {
            self.active_tab_id = *self.virtual_window_manager.get_tab_ids().first().unwrap();
        }
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

        // 操作対象のシェルを更新
        if let Some(shells) = self.active_shell_table.get_mut(&self.active_tab_id) {
            shells.push(shell_id);
        } else {
            self.active_shell_table
                .insert(self.active_tab_id, vec![shell_id]);
        }

        // 分割した下側
        // TileId を新たに発行して分割したウィンドウに紐付ける
        let tile_id = TileId {
            internal: new_virtual_window_id,
        };
        self.tile_shell_table.insert(tile_id, shell_id);
        self.id_set.insert(shell_id);

        tile_id
    }

    pub fn send_input(&mut self, input: &str) {
        let Some(active_shell_id) = self.active_shell_table.get(&self.active_tab_id) else {
            return;
        };

        self.shell_manager
            .send_input(*active_shell_id.last().unwrap(), input);
    }

    #[allow(dead_code)]
    pub fn send_input_specified(&mut self, input: &str, id: TileId) {
        let Some(shell_id) = self.tile_shell_table.get(&id) else {
            return;
        };

        self.shell_manager.send_input(*shell_id, input);
    }

    pub fn enumerate_content(&self) -> impl Iterator<Item = TShellManager::Content> + '_ {
        let child_virtual_window_ids = self
            .virtual_window_manager
            .find_children(self.active_tab_id);

        let mut contents = Vec::default();
        for (index, virtual_window_id) in child_virtual_window_ids
            .iter()
            .enumerate()
            .map(|(index, id)| (index, *id))
        {
            let tile_id = TileId {
                internal: virtual_window_id,
            };
            let Some(shell_id) = self.tile_shell_table.get(&tile_id) else {
                continue;
            };

            let height = (0..index)
                .map(|_| {
                    let tile_id = TileId {
                        internal: child_virtual_window_ids[0],
                    };
                    let Some(shell_id) = self.tile_shell_table.get(&tile_id) else {
                        return 0;
                    };

                    self.shell_manager.size(*shell_id).unwrap().1
                })
                .sum();

            let content = self.shell_manager.enumerate_content(*shell_id).map(|c| {
                let offset = TShellManager::Position::new(0, height);
                c.with_offset(&offset)
            });
            contents.extend(content);
        }
        contents.into_iter()
    }

    pub fn activate_tab(&mut self, index: u32) {
        let Some(id) = self
            .virtual_window_manager
            .get_tab_ids()
            .get(index as usize)
        else {
            // 範囲外を指定されたらタブを作成する使用にしてみる
            let new_tab_id = self.virtual_window_manager.spawn_virtual_window(1280, 640);
            let new_virtual_window_id = *self
                .virtual_window_manager
                .find_children(new_tab_id)
                .first()
                .unwrap();

            // 新規に追加した仮想ウィンドウに割り当てるシェルを起動
            let shell_id = self.shell_manager.spawn();
            self.active_shell_table.insert(new_tab_id, vec![shell_id]);

            // TileId を新たに発行して分割したウィンドウに紐付ける
            let tile_id = TileId {
                internal: new_virtual_window_id,
            };
            self.tile_shell_table.insert(tile_id, shell_id);
            self.id_set.insert(shell_id);
            self.active_tab_id = new_tab_id;
            return;
        };

        self.active_tab_id = *id;
    }

    #[allow(dead_code)]
    pub fn get_active_tab_id(&self) -> TabId {
        self.active_tab_id
    }

    #[allow(dead_code)]
    pub fn get_tab_ids(&self) -> &[TabId] {
        self.virtual_window_manager.get_tab_ids()
    }

    pub fn get_cursor_position(&self) -> (u32, u32) {
        let Some(shell_id) = self.active_shell_table.get(&self.active_tab_id) else {
            return (0, 0);
        };
        self.shell_manager
            .get_cursor_position(*shell_id.last().unwrap())
    }

    pub fn is_empty(&self) -> bool {
        self.id_set.is_empty()
    }

    pub fn consume_dirty(&mut self) -> Option<bool> {
        let shell_ids = self.active_shell_table.get(&self.active_tab_id)?;

        let mut is_dirty = false;
        for shell_id in shell_ids {
            let dirty = self.shell_manager.consume_dirty(*shell_id)?;
            is_dirty |= dirty;
        }

        Some(is_dirty)
    }
}
