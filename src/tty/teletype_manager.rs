use alacritty_terminal::event_loop::{EventLoopSender, State};
use alacritty_terminal::term::RenderableContent;
use alacritty_terminal::tty::{Options, Pty, Shell};
use alacritty_terminal::Term;
use alacritty_terminal::{
    event::{EventListener, WindowSize},
    event_loop::EventLoop,
    grid::Dimensions,
    sync::FairMutex,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeletypeId {
    internal: u64,
}

pub struct TeletypeManager {
    terminal_table: HashMap<TeletypeId, Arc<FairMutex<Term<EventProxy>>>>,
    io_handle_table: HashMap<TeletypeId, JoinHandle<(EventLoop<Pty, EventProxy>, State)>>,
    dirty_table: Arc<Mutex<HashMap<TeletypeId, bool>>>,
    current_id: u64,
}

impl TeletypeManager {
    pub fn new() -> Self {
        Self {
            terminal_table: Default::default(),
            io_handle_table: HashMap::default(),
            dirty_table: Arc::new(Mutex::new(HashMap::default())),
            current_id: 0,
        }
    }

    pub fn update(&mut self) {
        let mut finished_id = Vec::default();
        for (id, handle) in &self.io_handle_table {
            if !handle.is_finished() {
                continue;
            }

            finished_id.push(*id);
        }

        for id in finished_id {
            self.io_handle_table.remove(&id);
        }
    }

    pub fn create_teletype(&mut self) -> (TeletypeId, EventLoopSender) {
        self.create_teletype_with_size(SizeInfo::new())
    }

    pub fn create_teletype_with_size<TDimension>(
        &mut self,
        size: TDimension,
    ) -> (TeletypeId, EventLoopSender)
    where
        TDimension: Dimensions,
    {
        let id = TeletypeId {
            internal: self.current_id,
        };
        self.current_id += 1;

        let pty_config = &Options {
            #[cfg(not(target_os = "windows"))]
            shell: Some(Shell::new("bash".to_string(), Vec::default())),
            #[cfg(target_os = "windows")]
            shell: Some(Shell::new("cmd.exe".to_string(), Vec::default())),
            working_directory: None,
            hold: true,
        };
        let window_size = WindowSize {
            num_lines: 64,
            num_cols: 64,
            cell_width: 8,
            cell_height: 8,
        };

        let pty = alacritty_terminal::tty::new(pty_config, window_size, id.internal).unwrap();

        self.dirty_table.lock().unwrap().insert(id, true);
        let event_proxy = EventProxy::new(id, self.dirty_table.clone());
        let terminal =
            alacritty_terminal::Term::new(Default::default(), &size, event_proxy.clone());
        let terminal = Arc::new(FairMutex::new(terminal));

        let event_loop = EventLoop::new(
            Arc::clone(&terminal),
            event_proxy,
            pty,
            true, /*hold*/
            true, /*ref_test*/
        );
        // コマンドを送信するにはこれを返り値として渡す
        let channel = event_loop.channel();

        // 起動
        let io_thread = event_loop.spawn();
        self.io_handle_table.insert(id, io_thread);
        self.terminal_table.insert(id, terminal);

        (id, channel)
    }

    pub fn is_dirty(&self, id: TeletypeId) -> bool {
        *self.dirty_table.lock().unwrap().get(&id).unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.io_handle_table.is_empty()
    }

    pub fn clear_dirty(&mut self, id: TeletypeId) {
        *self.dirty_table.lock().unwrap().get_mut(&id).unwrap() = false;
    }

    pub fn get_content<TFunc: FnMut(RenderableContent)>(&self, id: TeletypeId, mut func: TFunc) {
        let terminal = self.terminal_table.get(&id).unwrap().lock();
        // let terminal = terminal.unwrap();
        // let terminal = terminal.lock();
        func(terminal.renderable_content());
    }
}

struct EventProxy {
    id: TeletypeId,
    dirty_table: Arc<Mutex<HashMap<TeletypeId, bool>>>,
}

impl EventProxy {
    pub fn new(id: TeletypeId, dirty_table: Arc<Mutex<HashMap<TeletypeId, bool>>>) -> Self {
        Self { dirty_table, id }
    }
}

impl EventListener for EventProxy {
    fn send_event(&self, event: alacritty_terminal::event::Event) {
        match event {
            alacritty_terminal::event::Event::Wakeup => {
                self.dirty_table.lock().unwrap().insert(self.id, true);
            }
            alacritty_terminal::event::Event::PtyWrite(str) => {
                // self.dirty_table.lock().unwrap().insert(self.id, true);
                println!("{}", str);
            }
            alacritty_terminal::event::Event::Bell => {
                // とりあえず未サポート
            }
            alacritty_terminal::event::Event::Exit => {
                // とくになにもしない
            }
            _ => {
                println!("{:?}", event)
            } // alacritty_terminal::event::Event::MouseCursorDirty => todo!(),
              // alacritty_terminal::event::Event::Title(_) => todo!(),
              // alacritty_terminal::event::Event::ResetTitle => todo!(),
              // alacritty_terminal::event::Event::ClipboardStore(_, _) => todo!(),
              // alacritty_terminal::event::Event::ClipboardLoad(_, _) => todo!(),
              // alacritty_terminal::event::Event::ColorRequest(_, _) => todo!(),
              // alacritty_terminal::event::Event::TextAreaSizeRequest(_) => todo!(),
              // alacritty_terminal::event::Event::CursorBlinkingChange => todo!(),
        }
    }
}

impl Clone for EventProxy {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            dirty_table: Arc::clone(&self.dirty_table),
        }
    }
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
