use crate::{ShellId, TabId};

pub struct TerminalEmulator {}

impl TerminalEmulator {
    pub fn new(_width: u32, _height: u32) -> (ShellId, Self) {
        todo!()
    }

    /// タブを生成します
    pub fn spawn_tab(&mut self) -> TabId {
        TabId::new()
    }

    /// ターミナル領域をリサイズします
    pub fn resize(&mut self, _width: u32, _height: u32) {
        todo!()
    }

    /// ターミナル内の特定の領域が占める割合を設定します
    pub fn resize_tile(&mut self, _id: ShellId, _ratio: f32) {
        todo!()
    }
}
