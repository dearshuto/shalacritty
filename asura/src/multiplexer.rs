use std::{
    borrow::Cow,
    collections::HashMap,
    sync::mpsc::{RecvError, TryRecvError},
    time::Duration,
};

use alacritty_terminal::{
    event::WindowSize,
    event_loop::{EventLoopSender, Msg},
    grid::Dimensions,
};

use crate::{
    detail::{TeletypeManagerEx, TerminalAccessor, TerminalProxy},
    shell_id::ShellId,
};

pub struct ShellController {
    // ターミナルの更新イベントの receiver
    // alacritty_terminal モジュールが asura 経由で外部ににじみ出ないように隠蔽している
    event_receiver: std::sync::mpsc::Receiver<alacritty_terminal::event::Event>,

    // ターミナルに処理を送る sender
    // セッターの役割
    input_sender: alacritty_terminal::event_loop::EventLoopSender,

    // ターミナルの情報にアクセスするためのインスタンス
    // ゲッターの役割
    proxy: TerminalProxy,
}

impl ShellController {
    pub fn is_running(&self) -> bool {
        let Err(error) = self.event_receiver.recv_timeout(Duration::from_nanos(1)) else {
            return true;
        };

        match error {
            std::sync::mpsc::RecvTimeoutError::Timeout => true,
            std::sync::mpsc::RecvTimeoutError::Disconnected => false,
        }
    }

    pub fn recv_event(&self) -> Result<(), RecvError> {
        match self.event_receiver.recv() {
            Ok(_event) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn try_recv_event(&self) -> Result<(), TryRecvError> {
        match self.event_receiver.try_recv() {
            Ok(_) => return Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn read_contents(&self) -> TerminalAccessor {
        self.proxy.read_lock()
    }

    pub fn send_input(&mut self, str: &str) {
        let bytes: Vec<_> = str.bytes().collect();
        self.input_sender
            .send(Msg::Input(Cow::Owned(bytes)))
            .unwrap_or_default();
    }

    pub fn resize(
        &mut self,
        line_count: u16,
        column_count: u16,
        cell_width: u16,
        cell_height: u16,
    ) {
        self.input_sender
            .send(Msg::Resize(WindowSize {
                num_lines: line_count,
                num_cols: column_count,
                cell_width,
                cell_height,
            }))
            .unwrap_or_default();
    }
}

pub struct Multiplexer {
    sender_table: HashMap<ShellId, EventLoopSender>,
    teletype_manager_ex: TeletypeManagerEx,
}

impl Multiplexer {
    pub fn new() -> Self {
        Self {
            teletype_manager_ex: TeletypeManagerEx::new(),
            sender_table: HashMap::default(),
        }
    }

    pub fn spawn(&mut self, config: &Config) -> (ShellId, ShellController) {
        let screen_lines = config.screen_lines;
        let dimension = Dimension {
            total_lines: config.total_lines,
            screen_lines,
            columns: config.columns,
        };
        let windows_size = WindowSize {
            num_lines: 64,
            num_cols: 80,
            cell_width: 8,
            cell_height: 8,
        };
        let teletype_data = self
            .teletype_manager_ex
            .create_teletype_with_size(dimension, windows_size);

        let id = ShellId::new(teletype_data.id);

        let sender = teletype_data.input_sender.clone();
        self.sender_table.insert(id, sender);

        (
            id,
            ShellController {
                event_receiver: teletype_data.event_receiver,
                input_sender: teletype_data.input_sender,
                proxy: teletype_data.proxy,
            },
        )
    }
}

impl Drop for Multiplexer {
    fn drop(&mut self) {
        // シェルの全破棄
        for sender in self.sender_table.values_mut() {
            sender.send(Msg::Shutdown).unwrap_or_default();
        }
    }
}

pub struct Config {
    total_lines: usize,

    screen_lines: usize,

    columns: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            total_lines: 64,
            screen_lines: 64,
            columns: 80,
        }
    }
}

impl Config {
    pub fn with_total_lines(mut self, total_line: usize) -> Self {
        self.total_lines = total_line;
        self
    }

    pub fn with_screen_lines(mut self, screen_line: usize) -> Self {
        self.screen_lines = screen_line;
        self
    }

    pub fn with_columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }
}

struct Dimension {
    total_lines: usize,
    screen_lines: usize,
    columns: usize,
}

impl From<&Config> for Dimension {
    fn from(value: &Config) -> Self {
        Self {
            total_lines: value.total_lines,
            screen_lines: value.screen_lines,
            columns: value.columns,
        }
    }
}

impl Dimensions for Dimension {
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
