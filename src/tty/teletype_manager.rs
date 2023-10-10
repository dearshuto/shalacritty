use std::{collections::HashMap, sync::Arc};

use alacritty_terminal::{
    event::{EventListener, WindowSize},
    grid::Dimensions,
    sync::FairMutex,
    Term,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeletypeId {
    id: uuid::Uuid,
}

impl TeletypeId {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
        }
    }
}

pub struct TeletypeManager {
    tty_table: HashMap<TeletypeId, alacritty_terminal::tty::Pty>,
    terminal_table: HashMap<TeletypeId, Arc<FairMutex<Term<EventProxy>>>>,
}

impl TeletypeManager {
    pub fn new() -> Self {
        Self {
            tty_table: HashMap::default(),
            terminal_table: Default::default(),
        }
    }

    pub fn create_teletype(&mut self) -> TeletypeId {
        let id = TeletypeId::new();
        let pty_config = &Default::default();
        let window_size = WindowSize {
            num_lines: 10,
            num_cols: 10,
            cell_width: 10,
            cell_height: 10,
        };

        // 紐づいた Window を表す識別子
        // とりあえず適当な数値で決め打ち
        let internal_id = 1;
        let pty = alacritty_terminal::tty::new(pty_config, window_size, internal_id).unwrap();
        self.tty_table.insert(id.clone(), pty);

        let event_proxy = EventProxy::new();

        let grid = SizeInfo::new();
        let terminal =
            alacritty_terminal::Term::new(&Default::default(), &grid, event_proxy.clone());
        let terminal = Arc::new(FairMutex::new(terminal));
        self.terminal_table.insert(id.clone(), terminal);

        id
    }
}

#[derive(Clone)]
struct EventProxy;

impl EventProxy {
    pub fn new() -> Self {
        Self {}
    }
}

impl EventListener for EventProxy {
    fn send_event(&self, _event: alacritty_terminal::event::Event) {}
}

struct SizeInfo;

impl SizeInfo {
    pub fn new() -> Self {
        Self {}
    }
}

impl Dimensions for SizeInfo {
    fn total_lines(&self) -> usize {
        64
    }

    fn screen_lines(&self) -> usize {
        64
    }

    fn columns(&self) -> usize {
        64
    }
}
