use std::collections::{HashMap, HashSet};

use crate::{
    detail::{TileId, VirtualWindow},
    Config, Multiplexer, ShellController, ShellId, TabId,
};

pub struct TerminalEmulator {
    multiplexer: Multiplexer,

    shell_controller_table: HashMap<ShellId, ShellController>,

    tab_table: HashMap<TabId, VirtualWindow>,

    tab_shell_table: HashMap<TabId, HashSet<ShellId>>,

    shell_tile_table: HashMap<ShellId, TileId>,
}

impl TerminalEmulator {
    pub fn new() -> (ShellId, Self) {
        Self::new_with(640, 480)
    }

    pub fn new_with(width: u32, height: u32) -> (ShellId, Self) {
        let mut multiplexer = Multiplexer::new();

        let (shell_id, controller) = multiplexer.spawn(&Config::default());

        let tab_id = TabId::new();
        let (tile_id, vw) = VirtualWindow::new(width, height);
        let tab_table = HashMap::from([(tab_id, vw)]);

        let shell_controller_table = HashMap::from([(shell_id, controller)]);
        let tab_shell_table = HashMap::from([(tab_id, HashSet::from([shell_id]))]);
        let shell_tile_table = HashMap::from([(shell_id, tile_id)]);

        (
            shell_id,
            Self {
                multiplexer,
                shell_controller_table,
                tab_table,
                tab_shell_table,
                shell_tile_table,
            },
        )
    }

    /// タブを生成します
    pub fn spawn_tab(&mut self) -> (TabId, ShellId) {
        let (shell_id, controller) = self.multiplexer.spawn(&Config::default());

        let width = 640;
        let height = 480;

        let tab_id = TabId::new();
        let (tile_id, vw) = VirtualWindow::new(width, height);

        self.shell_controller_table.insert(shell_id, controller);
        self.tab_table.insert(tab_id, vw);
        self.tab_shell_table
            .insert(tab_id, HashSet::from([shell_id]));
        self.shell_tile_table.insert(shell_id, tile_id);

        (tab_id, shell_id)
    }

    /// ターミナル領域をリサイズします
    pub fn resize(&mut self, width: u32, height: u32) {
        // すべてのタブにリサイズを反映
        for vw in self.tab_table.values_mut() {
            vw.resize(width, height);
        }

        // シェルにリサイズを反映
        for (tab_id, window) in &self.tab_table {
            let Some(shell_ids) = self.tab_shell_table.get(tab_id) else {
                continue;
            };

            for shell_id in shell_ids {
                let Some(tile_id) = self.shell_tile_table.get(shell_id) else {
                    continue;
                };

                let Some(actual_size) = window.get_actual_size(*tile_id) else {
                    continue;
                };

                let Some(controller) = self.shell_controller_table.get_mut(shell_id) else {
                    continue;
                };

                let line_count = actual_size.width as u16;
                let column_count = actual_size.height as u16;
                controller.resize(line_count, column_count, 8, 8);
            }
        }
    }
}
