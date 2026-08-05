use std::collections::HashMap;
use std::sync::Arc;

use alacritty_terminal::event::EventListener;
use alacritty_terminal::event::WindowSize;
use alacritty_terminal::event_loop::EventLoop;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::tty::Options;
use alacritty_terminal::tty::Pty;
use alacritty_terminal::tty::Shell;
use eframe::egui::DragAndDrop;

pub trait EventProxy: Clone {}

pub struct Terminal<T>
where
    T: EventProxy + Send,
{
    event_loop: EventLoop<Pty, EventListenerBridge<T>>,
}

impl<T> Terminal<T>
where
    T: EventProxy + Send + 'static,
{
    pub fn new(event_proxy: T) -> Self {
        let pty_config = &Options {
            #[cfg(not(target_os = "windows"))]
            shell: Some(Shell::new("bash".to_string(), Vec::default())),
            #[cfg(target_os = "windows")]
            shell: Some(Shell::new("cmd.exe".to_string(), Vec::default())),
            working_directory: None,
            env: HashMap::default(),
            drain_on_exit: true,
        };
        let window_size = WindowSize {
            num_lines: 64,
            num_cols: 64,
            cell_width: 8,
            cell_height: 8,
        };

        let window_id = 123;
        let pty = alacritty_terminal::tty::new(pty_config, window_size, window_id).unwrap();

        let event_proxy = EventListenerBridge {
            internal: event_proxy,
        };
        let terminal = alacritty_terminal::Term::new(
            Default::default(),
            &SizeInfo::new(),
            event_proxy.clone(),
        );
        let event_loop = EventLoop::new(
            Arc::new(FairMutex::new(terminal)),
            event_proxy,
            pty,
            true,
            true,
        )
        .unwrap();
        // let sender = event_loop.channel();

        Self { event_loop }
    }

    pub fn run(self) {
        // self.event_loop.spawn().join().unwrap();
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

#[derive(Debug, Clone)]
struct EventListenerBridge<T>
where
    T: EventProxy + Clone + Send,
{
    internal: T,
}

impl<T> EventListener for EventListenerBridge<T>
where
    T: EventProxy + Clone + Send,
{
    fn send_event(&self, event: alacritty_terminal::event::Event) {
        match event {
            alacritty_terminal::event::Event::MouseCursorDirty => todo!(),
            alacritty_terminal::event::Event::Title(_) => todo!(),
            alacritty_terminal::event::Event::ResetTitle => todo!(),
            alacritty_terminal::event::Event::ClipboardStore(clipboard_type, _) => todo!(),
            alacritty_terminal::event::Event::ClipboardLoad(clipboard_type, _) => todo!(),
            alacritty_terminal::event::Event::ColorRequest(_, _) => todo!(),
            alacritty_terminal::event::Event::PtyWrite(_) => todo!(),
            alacritty_terminal::event::Event::TextAreaSizeRequest(_) => todo!(),
            alacritty_terminal::event::Event::CursorBlinkingChange => todo!(),
            alacritty_terminal::event::Event::Wakeup => todo!(),
            alacritty_terminal::event::Event::Bell => todo!(),
            alacritty_terminal::event::Event::Exit => todo!(),
            alacritty_terminal::event::Event::ChildExit(exit_status) => todo!(),
        }
    }
}
