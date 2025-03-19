use std::{collections::HashMap, sync::Arc, thread::JoinHandle};

use alacritty_terminal::{
    event::{EventListener, WindowSize},
    event_loop::{EventLoop, State},
    grid::Dimensions,
    sync::FairMutex,
    term::color::Colors,
    tty::{Options, Pty, Shell},
    vte::ansi::Color,
};

use parking_lot::MutexGuard;

use super::TeletypeId;

pub struct Content {
    pub code: char,
    pub x: usize,
    pub y: i32,
    pub fg: [f32; 3],
}

pub struct TerminalAccessor<'a> {
    internal: MutexGuard<'a, alacritty_terminal::Term<EventProxy>>,
}

impl<'a> TerminalAccessor<'a> {
    pub fn get_cursor_point(&self) -> (usize, i32) {
        let point = self.internal.renderable_content().cursor.point;
        (point.column.0, point.line.0)
    }

    pub fn acquire_contents(&self) -> Vec<Content> {
        self.internal
            .renderable_content()
            .display_iter
            .map(|content| {
                let fg = convert_color(&content.fg, self.internal.colors());

                Content {
                    code: content.c,
                    x: content.point.column.0,
                    y: content.point.line.0,
                    fg,
                }
            })
            .collect()
    }
}

pub struct TerminalProxy {
    internal: Arc<FairMutex<alacritty_terminal::Term<EventProxy>>>,
}

impl TerminalProxy {
    pub fn read_lock(&mut self) -> TerminalAccessor {
        TerminalAccessor {
            internal: self.internal.lock(),
        }
    }
}

pub struct TeletypeData {
    pub id: TeletypeId,
    pub event_receiver: std::sync::mpsc::Receiver<alacritty_terminal::event::Event>,
    pub input_sender: alacritty_terminal::event_loop::EventLoopSender,
    pub proxy: TerminalProxy,
}

pub struct TeletypeManagerEx {
    current_id: u64,

    thread_table: HashMap<TeletypeId, JoinHandle<(EventLoop<Pty, EventProxy>, State)>>,
}

impl TeletypeManagerEx {
    pub fn new() -> Self {
        Self {
            current_id: 0,
            thread_table: HashMap::default(),
        }
    }

    pub fn create_teletype_with_size<TDimension>(
        &mut self,
        dimension: TDimension,
        window_size: WindowSize,
    ) -> TeletypeData
    where
        TDimension: Dimensions,
    {
        let id = self.generate_teletype_id();

        let pty_config = &Options {
            #[cfg(not(target_os = "windows"))]
            shell: Some(Shell::new("bash".to_string(), Vec::default())),
            #[cfg(target_os = "windows")]
            shell: Some(Shell::new("cmd.exe".to_string(), Vec::default())),
            working_directory: None,
            env: HashMap::default(),
            drain_on_exit: true,
        };

        let pty = alacritty_terminal::tty::new(pty_config, window_size, id.internal).unwrap();

        let (sender, receiver) = std::sync::mpsc::channel();
        let event_proxy = EventProxy { sender };
        let terminal =
            alacritty_terminal::Term::new(Default::default(), &dimension, event_proxy.clone());
        let terminal = Arc::new(FairMutex::new(terminal));

        let event_loop = EventLoop::new(
            terminal.clone(),
            event_proxy,
            pty,
            true, /*hold*/
            true, /*ref_test*/
        )
        .unwrap();
        // コマンドを送信するにはこれを返り値として渡す
        let input_sender = event_loop.channel();

        // 起動
        let io_thread = event_loop.spawn();
        self.thread_table.insert(id, io_thread);

        TeletypeData {
            id,
            event_receiver: receiver,
            input_sender,
            proxy: TerminalProxy { internal: terminal },
        }
    }

    fn generate_teletype_id(&mut self) -> TeletypeId {
        let id = TeletypeId {
            internal: self.current_id,
        };
        self.current_id += 1;
        id
    }
}

impl Drop for TeletypeManagerEx {
    fn drop(&mut self) {
        // 全スレッドの終了待機
        for (_key, value) in self.thread_table.drain() {
            value.join().unwrap();
        }
    }
}

#[derive(Clone)]
struct EventProxy {
    sender: std::sync::mpsc::Sender<alacritty_terminal::event::Event>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: alacritty_terminal::event::Event) {
        self.sender.send(event).unwrap();
    }
}

fn convert_color(color: &Color, colors: &Colors) -> [f32; 3] {
    let rgb = match color {
        &alacritty_terminal::vte::ansi::Color::Named(named_color) => colors[named_color],
        &alacritty_terminal::vte::ansi::Color::Spec(rgb) => Some(rgb),
        &alacritty_terminal::vte::ansi::Color::Indexed(index) => colors[index as usize],
    }
    .unwrap_or_default();

    let r = rgb.r as f32 / u8::MAX as f32;
    let g = rgb.g as f32 / u8::MAX as f32;
    let b = rgb.b as f32 / u8::MAX as f32;
    [r, g, b]
}
