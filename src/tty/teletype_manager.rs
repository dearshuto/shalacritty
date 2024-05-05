use alacritty_terminal::event_loop::{EventLoopSender, Msg, State};
use alacritty_terminal::index::Point;
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

pub struct TeletypeHandle {
    id: TeletypeId,
    dirty_flag: Arc<Mutex<bool>>,
    event_loop_sender: EventLoopSender,
}

impl TeletypeHandle {
    pub fn id(&self) -> TeletypeId {
        self.id
    }

    pub fn consume_dirty(&mut self) -> Option<bool> {
        let Ok(mut value) = self.dirty_flag.lock() else {
            return None;
        };
        let is_dirty = *value;
        *value = false;

        Some(is_dirty)
    }

    pub fn send(&self, message: Msg) {
        self.event_loop_sender.send(message).unwrap();
    }
}

pub struct TeletypeManager {
    terminal_table: HashMap<TeletypeId, Arc<FairMutex<Term<EventProxy>>>>,
    io_handle_table: HashMap<TeletypeId, JoinHandle<(EventLoop<Pty, EventProxy>, State)>>,
    ptr_write_table: Arc<Mutex<HashMap<TeletypeId, Vec<u8>>>>,
    current_id: u64,
}

impl TeletypeManager {
    pub fn new() -> Self {
        Self {
            terminal_table: Default::default(),
            io_handle_table: HashMap::default(),
            ptr_write_table: Arc::new(Mutex::new(HashMap::default())),
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

    pub fn create_teletype(&mut self) -> TeletypeHandle {
        self.create_teletype_with_size(SizeInfo::new())
    }

    pub fn create_teletype_with_size<TDimension>(&mut self, size: TDimension) -> TeletypeHandle
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

        let dirty_flag = Arc::new(Mutex::new(false));
        let event_proxy = EventProxy::new(id, dirty_flag.clone(), self.ptr_write_table.clone());
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

        TeletypeHandle {
            id,
            dirty_flag,
            event_loop_sender: channel,
        }
    }

    pub fn consume_ptr_write(&self) -> Vec<Vec<u8>> {
        self.terminal_table
            .keys()
            .filter_map(|id| self.ptr_write_table.lock().unwrap().remove(&id))
            .collect()
    }

    // シェルが勝手に閉じた場合に対応するためにこの関数を参照した方がよい
    // ただ現状は上手に使えてないので allow している
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.io_handle_table.is_empty()
    }

    pub fn contains(&self, id: TeletypeId) -> bool {
        self.io_handle_table.contains_key(&id)
    }

    pub fn get_content<TFunc: FnMut(RenderableContent)>(&self, id: TeletypeId, mut func: TFunc) {
        let terminal = self.terminal_table.get(&id).unwrap().lock();
        // let terminal = terminal.unwrap();
        // let terminal = terminal.lock();
        func(terminal.renderable_content());
    }

    pub fn get_cursor(&self, id: TeletypeId) -> Point {
        let terminal = self.terminal_table.get(&id).unwrap().lock();
        terminal.renderable_content().cursor.point
    }

    pub fn resize(&mut self, id: TeletypeId, width: u32, height: u32) {
        let Some(term) = self.terminal_table.get(&id) else {
            return;
        };

        let line = height as usize / 16;
        let columns = width as usize / 16;
        term.lock()
            .resize(SizeInfo::new_with(128 /*total*/, line, columns));
    }

    pub fn size(&self, id: TeletypeId) -> Option<(u32, u32)> {
        let term = self.terminal_table.get(&id)?;

        let binding = term.lock();
        let column = binding.columns();
        let lines = binding.screen_lines();
        Some((column as u32, lines as u32))
    }
}

struct EventProxy {
    id: TeletypeId,
    dirty_flag: Arc<Mutex<bool>>,
    ptr_write_table: Arc<Mutex<HashMap<TeletypeId, Vec<u8>>>>,
}

impl EventProxy {
    pub fn new(
        id: TeletypeId,
        dirty_flag: Arc<Mutex<bool>>,
        ptr_write_table: Arc<Mutex<HashMap<TeletypeId, Vec<u8>>>>,
    ) -> Self {
        Self {
            dirty_flag,
            id,
            ptr_write_table,
        }
    }
}

impl EventListener for EventProxy {
    fn send_event(&self, event: alacritty_terminal::event::Event) {
        match event {
            alacritty_terminal::event::Event::Wakeup => {
                *self.dirty_flag.lock().unwrap() = true;
            }
            alacritty_terminal::event::Event::PtyWrite(str) => {
                self.ptr_write_table
                    .lock()
                    .unwrap()
                    .insert(self.id, str.into_bytes());
            }
            alacritty_terminal::event::Event::Bell => {
                // とりあえず未サポート
            }
            alacritty_terminal::event::Event::Exit => {
                // とくになにもしない
            }
            alacritty_terminal::event::Event::CursorBlinkingChange => {
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
        }
    }
}

impl Clone for EventProxy {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            dirty_flag: Arc::clone(&self.dirty_flag),
            ptr_write_table: Arc::clone(&self.ptr_write_table),
        }
    }
}

struct SizeInfo {
    total_lines: usize,
    screen_lines: usize,
    columns: usize,
}

impl SizeInfo {
    pub fn new() -> Self {
        Self {
            total_lines: 64,
            screen_lines: 64,
            columns: 64,
        }
    }

    pub fn new_with(t: usize, s: usize, c: usize) -> Self {
        Self {
            total_lines: t,
            screen_lines: s,
            columns: c,
        }
    }
}

impl Dimensions for SizeInfo {
    fn total_lines(&self) -> usize {
        self.total_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}
