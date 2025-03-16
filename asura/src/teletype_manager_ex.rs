use std::{collections::HashMap, sync::Arc, thread::JoinHandle};

use alacritty_terminal::{
    event::{EventListener, WindowSize},
    event_loop::{EventLoop, State},
    grid::Dimensions,
    sync::FairMutex,
    tty::{Options, Pty, Shell},
};

use super::TeletypeId;

pub struct TeletypeData {
    pub id: TeletypeId,
    pub event_receiver: std::sync::mpsc::Receiver<alacritty_terminal::event::Event>,
    pub input_sender: alacritty_terminal::event_loop::EventLoopSender,
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
            terminal,
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
