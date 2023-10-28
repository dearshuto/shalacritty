use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alacritty_terminal::config::{Program, PtyConfig};
use alacritty_terminal::event_loop::EventLoopSender;
use alacritty_terminal::term::RenderableContent;
use alacritty_terminal::Term;
use alacritty_terminal::{
    event::{EventListener, WindowSize},
    event_loop::EventLoop,
    grid::Dimensions,
    sync::FairMutex,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeletypeId {
    internal: u64,
}

pub struct TeletypeManager {
    terminal_table: HashMap<TeletypeId, Arc<FairMutex<Term<EventProxy>>>>,
    dirty_table: Arc<Mutex<HashMap<TeletypeId, bool>>>,
    current_id: u64,
}

impl TeletypeManager {
    pub fn new() -> Self {
        Self {
            terminal_table: Default::default(),
            dirty_table: Arc::new(Mutex::new(HashMap::default())),
            current_id: 0,
        }
    }

    pub fn create_teletype(&mut self) -> (TeletypeId, EventLoopSender) {
        let id = TeletypeId {
            internal: self.current_id,
        };
        self.current_id += 1;

        let pty_config = &PtyConfig {
            shell: Some(Program::Just("bash".to_string())),
            working_directory: None,
            hold: true,
        };
        let window_size = WindowSize {
            num_lines: 128,
            num_cols: 128,
            cell_width: 8,
            cell_height: 8,
        };

        let pty = alacritty_terminal::tty::new(pty_config, window_size, id.internal).unwrap();

        let event_proxy = EventProxy::new(id.clone(), self.dirty_table.clone());
        let grid = SizeInfo::new();
        let terminal =
            alacritty_terminal::Term::new(&Default::default(), &grid, event_proxy.clone());
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
        let _io_thread = event_loop.spawn();
        self.terminal_table.insert(id.clone(), terminal);

        (id, channel)
    }

    pub fn is_dirty(&self, id: TeletypeId) -> bool {
        *self.dirty_table.lock().unwrap().get(&id).unwrap()
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
            alacritty_terminal::event::Event::PtyWrite(str) =>{
                // self.dirty_table.lock().unwrap().insert(self.id, true);
                println!("{}", str);
            },
             _ => {println!("{:?}", event)}
            // alacritty_terminal::event::Event::MouseCursorDirty => todo!(),
            // alacritty_terminal::event::Event::Title(_) => todo!(),
            // alacritty_terminal::event::Event::ResetTitle => todo!(),
            // alacritty_terminal::event::Event::ClipboardStore(_, _) => todo!(),
            // alacritty_terminal::event::Event::ClipboardLoad(_, _) => todo!(),
            // alacritty_terminal::event::Event::ColorRequest(_, _) => todo!(),
           // alacritty_terminal::event::Event::TextAreaSizeRequest(_) => todo!(),
            // alacritty_terminal::event::Event::CursorBlinkingChange => todo!(),
            // alacritty_terminal::event::Event::Bell => todo!(),
            // alacritty_terminal::event::Event::Exit => todo!(),
        }
    }
}

impl Clone for EventProxy {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
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
        128
    }

    fn columns(&self) -> usize {
        128
    }
}
