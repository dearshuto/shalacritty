use winit::event::KeyEvent;

use crate::{Terminal, terminal::DiffInfo};

pub struct TerminalAdapter {
    terminal: Terminal,
    sender: std::sync::mpsc::Sender<DiffInfo>,
}

impl TerminalAdapter {
    pub fn new(sender: std::sync::mpsc::Sender<DiffInfo>) -> Self {
        Self {
            terminal: Terminal::new(),
            sender,
        }
    }

    pub fn handle_event(&mut self, _event: &KeyEvent) {
        // TODO: キー入力のハンドリング
        let diff_info = self.terminal.push(&[]);
        self.sender.send(diff_info);
    }
}
