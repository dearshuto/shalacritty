use std::{borrow::Cow, collections::HashMap};

use alacritty_terminal::{
    event::WindowSize,
    event_loop::{EventLoopSender, Msg},
};

use crate::tty::{TeletypeId, TeletypeManager};

use super::multiplexers::IShellManager;

pub struct MultiplexersAdapter {
    teletype_manager: TeletypeManager,
    event_loop_sender_table: HashMap<TeletypeId, EventLoopSender>,
}

impl IShellManager for MultiplexersAdapter {
    type Id = TeletypeId;

    fn spawn(&mut self) -> Self::Id {
        let (id, event_loop_sender) = self.teletype_manager.create_teletype();
        self.event_loop_sender_table.insert(id, event_loop_sender);
        id
    }

    fn send_input(&mut self, id: Self::Id, _input: &str) {
        let Some(event_loop_sender) = self.event_loop_sender_table.get(&id) else {
            return;
        };

        event_loop_sender
            .send(Msg::Input(Cow::Borrowed(&[])))
            .unwrap();
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
}
