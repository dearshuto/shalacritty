use std::collections::{HashMap, HashSet};

use alacritty_terminal::{
    grid::Indexed,
    term::{cell::Cell, color::Colors},
};

use crate::{
    detail::{self, ContentCache, DiffStream, TileId, VirtualWindow},
    Config, Content, Multiplexer, ShellController, ShellId, TabId,
};

pub struct TerminalEmulator {
    multiplexer: Multiplexer,

    shell_controller_table: HashMap<ShellId, ShellController>,

    tab_table: HashMap<TabId, VirtualWindow>,

    tab_shell_table: HashMap<TabId, HashSet<ShellId>>,

    shell_tile_table: HashMap<ShellId, TileId>,

    dirty_table: HashMap<ShellId, bool>,
}

impl TerminalEmulator {
    pub fn new() -> (TabId, ShellId, Self) {
        Self::new_with(640, 480)
    }

    pub fn new_with(width: u32, height: u32) -> (TabId, ShellId, Self) {
        let mut multiplexer = Multiplexer::new();

        let (shell_id, controller) = multiplexer.spawn(&Config::default());

        let tab_id = TabId::new();
        let (tile_id, vw) = VirtualWindow::new(width, height);
        let tab_table = HashMap::from([(tab_id, vw)]);

        let shell_controller_table = HashMap::from([(shell_id, controller)]);
        let tab_shell_table = HashMap::from([(tab_id, HashSet::from([shell_id]))]);
        let shell_tile_table = HashMap::from([(shell_id, tile_id)]);

        (
            tab_id,
            shell_id,
            Self {
                multiplexer,
                shell_controller_table,
                tab_table,
                tab_shell_table,
                shell_tile_table,
                dirty_table: HashMap::from([(shell_id, false)]),
            },
        )
    }

    pub fn update(&mut self) {
        self.shell_controller_table
            .retain(|key, value| match value.try_recv_event() {
                Ok(event) => match event {
                    crate::multiplexer::Event::Updated => {
                        self.dirty_table.insert(*key, true);
                        true
                    }
                    crate::multiplexer::Event::Exit => false,
                    crate::multiplexer::Event::Others => {
                        self.dirty_table.insert(*key, false);
                        true
                    }
                },
                Err(error) => match error {
                    std::sync::mpsc::TryRecvError::Empty => {
                        self.dirty_table.insert(*key, false);
                        true
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => false,
                },
            });
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

                let line_count = actual_size.height as u16;
                let column_count = actual_size.width as u16;
                controller.resize(line_count, column_count, 8, 8);
            }
        }
    }

    pub fn send_input(&mut self, id: ShellId, input: &str) {
        let Some(controller) = self.shell_controller_table.get_mut(&id) else {
            return;
        };

        controller.send_input(input);
    }

    pub fn acquire_content_as_string(&self, id: ShellId) -> Option<String> {
        let Some(x) = self.shell_controller_table.get(&id) else {
            return None;
        };

        Some(
            x.read_contents()
                .acquire_contents()
                .iter()
                .map(|c| c.code)
                .collect(),
        )
    }

    pub fn acquire_content<T: FnOnce(&[Content], (usize, i32))>(&self, id: ShellId, f: T) {
        let Some(x) = self.shell_controller_table.get(&id) else {
            return;
        };

        let contents = x.read_contents();
        let cursor = contents.get_cursor_point();
        let contents = contents.acquire_contents();

        f(&contents, cursor);
    }

    /// 指定したシェルの表示要素の差分を検出します
    pub fn diff(&self, id: ShellId, context: &mut DiffContext) -> Diff {
        let Some(controller) = self.shell_controller_table.get(&id) else {
            return Default::default();
        };

        let mut diff_data = None;
        let mut cursor_position = (0, 0);
        controller
            .read_contents()
            .access_rendarable_content(|renderable_content| {
                cursor_position = (
                    renderable_content.cursor.point.column.0,
                    renderable_content.cursor.point.line.0,
                );

                let iter = renderable_content
                    .display_iter
                    .enumerate()
                    .map(|c| CellAdapter((c.0, c.1, renderable_content.colors)));
                diff_data = Some(context.diff_stream.calculate(iter));
            });

        let diff_data = diff_data.unwrap();
        Diff {
            content_diff: diff_data.diff_types,
            cursor_position,
            content_count: diff_data.item_count,
        }
    }

    pub fn acquire_cursor_position_tentative(&self, id: ShellId) -> Option<(usize, i32)> {
        let Some(controller) = self.shell_controller_table.get(&id) else {
            return None;
        };

        Some(controller.read_contents().get_cursor_point())
    }

    pub fn is_dirty(&self, id: ShellId) -> Option<bool> {
        self.dirty_table.get(&id).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.shell_controller_table.is_empty()
    }
}

pub struct DiffContext {
    diff_stream: DiffStream,
}

impl DiffContext {
    pub fn new() -> Self {
        Self {
            diff_stream: DiffStream::new(),
        }
    }
}

impl Default for DiffContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Diff {
    pub content_diff: Vec<DiffType>,
    pub cursor_position: (usize, i32),
    pub content_count: Option<usize>,
}

#[derive(Debug, PartialEq)]
pub enum DiffType {
    Add(DiffContent),
    Update(DiffContent),
    Remove(usize),
}

#[derive(Debug, PartialEq)]
pub struct DiffContent {
    pub index: usize,
    pub content: Content,
}

struct CellAdapter<'a>((usize, Indexed<&'a Cell>, &'a Colors));

impl<'a> Into<(ContentCache, DiffContent)> for CellAdapter<'a> {
    fn into(self) -> (ContentCache, DiffContent) {
        let (index, cell, colors) = self.0;

        let fg = detail::convert_color_uint(&cell.fg, colors);
        let fg_snorm = detail::into_snorm(&fg);
        let content_cache = ContentCache {
            code: cell.c,
            x: cell.point.column.0,
            y: cell.point.line.0,
            fg,
        };
        let diff_content = DiffContent {
            index,
            content: Content {
                code: cell.c,
                x: cell.point.column.0,
                y: cell.point.line.0,
                fg: fg_snorm,
            },
        };
        (content_cache, diff_content)
    }
}

impl<'a> PartialEq<ContentCache> for CellAdapter<'a> {
    fn eq(&self, other: &ContentCache) -> bool {
        let (_, cell, colors) = &self.0;

        if cell.c != other.code {
            return false;
        }

        if cell.point.column.0 != other.x {
            return false;
        }

        if cell.point.line.0 != other.y {
            return false;
        }

        let fg = detail::convert_color_uint(&cell.fg, colors);
        if fg != other.fg {
            return false;
        }

        return true;
    }
}
