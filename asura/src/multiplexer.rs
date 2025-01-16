use std::{borrow::Cow, collections::HashMap};

use crate::{
    detail::{TeletypeHandle, TeletypeManager},
    shell_id::ShellId,
    ShellEvent,
};

pub struct Multiplexer {
    teletype_manager: TeletypeManager,
    table: HashMap<ShellId, TeletypeHandle>,
}

impl Multiplexer {
    pub fn new() -> Self {
        Self {
            teletype_manager: TeletypeManager::new(),
            table: HashMap::default(),
        }
    }

    pub fn spawn(&mut self, _width: u32, _height: u32) -> ShellId {
        let handle = self.teletype_manager.create_teletype();
        let shell_id = ShellId::new();

        self.table.insert(shell_id, handle);

        shell_id
    }

    pub fn subscribe_shell_event(&mut self, id: ShellId) -> ShellEvent {
        // TODO
        ShellEvent {}
    }

    pub fn resize(&mut self, id: ShellId, width: u32, height: u32) {
        let Some(handle) = self.table.get(&id) else {
            return;
        };

        self.teletype_manager.resize(handle.id(), width, height);
    }

    // TODO: 暫定実装。将来的に API の設計は変える。
    // TODO: 表示要素をどのように外部と連携させるか設計を考える
    pub fn enumerate_content(&self, id: ShellId) -> Result<String, ()> {
        let Some(handle) = self.table.get(&id) else {
            return Err(());
        };

        let mut buffer = String::new();
        self.teletype_manager.get_content(handle.id(), |x| {
            let mut y = 0;
            for i in x.display_iter {
                // 行が変わったら改行コードを挿入
                if y < i.point.line.0 {
                    buffer.push('\n');
                    y = i.point.line.0;
                }
                buffer.push(i.c);
            }
        });

        // 末尾の空白を取り除く
        while let Some(last) = buffer.chars().last() {
            if last != ' ' {
                break;
            }
            let _ = buffer.pop();
        }

        Ok(buffer)
    }

    pub fn input(&mut self, id: ShellId, input: &[u8]) {
        let Some(handle) = self.table.get(&id) else {
            return;
        };

        let mut bytes = Vec::with_capacity(input.len() + 1);
        bytes.extend_from_slice(input);
        if input.is_empty() {
            bytes.push(b'\x1b');
        }

        handle.send(alacritty_terminal::event_loop::Msg::Input(Cow::Owned(
            bytes,
        )));
        self.teletype_manager.update();
    }
}
