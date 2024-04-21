use std::{borrow::Cow, collections::HashMap};

use alacritty_terminal::{
    event::WindowSize,
    event_loop::{EventLoopSender, Msg},
    grid::Indexed,
    term::cell::Cell,
};

use crate::{
    multiplexers::IShellManager,
    tty::{TeletypeId, TeletypeManager},
};

pub struct MultiplexersAdapter {
    teletype_manager: TeletypeManager,
    event_loop_sender_table: HashMap<TeletypeId, EventLoopSender>,
}

impl MultiplexersAdapter {
    pub fn new() -> Self {
        Self {
            teletype_manager: TeletypeManager::new(),
            event_loop_sender_table: HashMap::default(),
        }
    }
}

impl IShellManager for MultiplexersAdapter {
    type Id = TeletypeId;
    type Content = Indexed<Cell>;

    fn spawn(&mut self) -> Self::Id {
        let (id, event_loop_sender) = self.teletype_manager.create_teletype();
        self.event_loop_sender_table.insert(id, event_loop_sender);
        id
    }

    fn send_input(&mut self, id: Self::Id, input: &str) {
        let Some(event_loop_sender) = self.event_loop_sender_table.get(&id) else {
            return;
        };

        // workspace/mod.rs からコピった実装なのでここに集約すべし
        let mut bytes = Vec::with_capacity(input.len() + 1);
        bytes.extend_from_slice(input.as_bytes());
        if input.is_empty() {
            bytes.push(b'\x1b');
        }

        let send_data: std::borrow::Cow<[u8]> = match input {
            // 上
            "ArrowUp" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x41]),
            "\u{f700}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x41]),
            // 下
            "ArrowDown" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x42]),
            "\u{f701}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x42]),
            // 左
            "ArrowLeft" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x44]),
            "\u{f702}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x44]),
            // 右
            "ArrowRight" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x43]),
            "\u{f703}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x43]),
            _ => std::borrow::Cow::Owned(bytes),
        };

        event_loop_sender.send(Msg::Input(send_data)).unwrap();
    }

    fn resize(&mut self, id: Self::Id, _width: i32, _height: i32) {
        let Some(event_loop_sender) = self.event_loop_sender_table.get(&id) else {
            return;
        };

        event_loop_sender
            .send(Msg::Resize(WindowSize {
                num_lines: 64,
                num_cols: 64,
                cell_width: 64,
                cell_height: 64,
            }))
            .unwrap();
    }

    fn is_running(&self, id: Self::Id) -> bool {
        self.event_loop_sender_table.contains_key(&id)
    }

    fn enumerate_content(&self, id: Self::Id) -> impl Iterator<Item = Self::Content> {
        let mut contents = Vec::new();
        self.teletype_manager.get_content(id, |c| {
            contents = c
                .display_iter
                .map(|c| Indexed {
                    point: c.point,
                    cell: c.cell.clone(),
                })
                .collect();
        });
        contents.into_iter()
    }
}
