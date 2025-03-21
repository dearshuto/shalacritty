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
    detail::{
        TeletypeManagerEx, TerminalAccessor, TerminalProxy, VirtualWindowId, VirtualWindowManager,
    },
    shell_id::ShellId,
};

pub struct ShellController {
    // ターミナルの更新イベントの receiver
    // alacritty_terminal モジュールが asura 経由で外部ににじみ出ないように隠蔽している
    event_receiver: std::sync::mpsc::Receiver<alacritty_terminal::event::Event>,

    // ターミナルに処理を送る sender
    // セッターの役割
    //
    // sender を外部に公開はしないで、メソッドで必要な機能のみにアクセスできるようにする
    // 特に隠蔽したいのはリサイズの処理
    // multiplexer 実装で各シェルの表示領域を更新するタイミングを内部で管理するために隠蔽が必須
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
}

pub struct Multiplexer {
    sender_table: HashMap<ShellId, EventLoopSender>,
    virtual_window_manager: VirtualWindowManager,
    teletype_manager_ex: TeletypeManagerEx,
    virtual_window_table: HashMap<ShellId, VirtualWindowId>,

    window_size: (u32, u32),
}

impl Multiplexer {
    pub fn new() -> Self {
        Self {
            virtual_window_manager: VirtualWindowManager::new(),
            teletype_manager_ex: TeletypeManagerEx::new(),
            sender_table: HashMap::default(),
            virtual_window_table: HashMap::default(),
            window_size: (640, 480),
        }
    }

    pub fn spawn(&mut self, config: &Config) -> (ShellId, ShellController) {
        // TODO: 表示領域を算出する
        let root_vw = self.virtual_window_manager.ids().first().unwrap();
        let _root_vw = self
            .virtual_window_manager
            .try_get_virtual_window(*root_vw)
            .unwrap();

        let screen_lines = ((self.window_size.1 as f32 / 10.0) as usize).min(config.screen_lines);
        let dimension = Dimension {
            total_lines: config.total_lines,
            screen_lines,
            columns: config.columns,
        };
        let windows_size = Self::into_window_size(self.window_size.0, self.window_size.1, 10.0);
        let teletype_data = self
            .teletype_manager_ex
            .create_teletype_with_size(dimension, windows_size);

        let id = ShellId::new(teletype_data.id);

        let sender = teletype_data.input_sender.clone();
        self.sender_table.insert(id, sender);
        self.virtual_window_table.insert(id, *root_vw);

        (
            id,
            ShellController {
                event_receiver: teletype_data.event_receiver,
                input_sender: teletype_data.input_sender,
                proxy: teletype_data.proxy,
            },
        )
    }

    /// 表示可能な領域を更新します
    pub fn resize_window(&mut self, width: u32, height: u32) {
        self.virtual_window_manager.resize_root(width, height)
    }

    /// シェルを表示する領域を更新します
    pub fn resize_shell(&mut self, id: ShellId, width: u32, height: u32) {
        let Some(vw_id) = self.virtual_window_table.get(&id) else {
            return;
        };

        let Some(sender) = self.sender_table.get(&id) else {
            return;
        };

        self.virtual_window_manager.resize(*vw_id, width, height);

        let Some(vw) = self.virtual_window_manager.try_get_virtual_window(*vw_id) else {
            return;
        };

        let window_size = Self::into_window_size(vw.width(), vw.height(), 12.0);
        sender.send(Msg::Resize(window_size)).unwrap_or_default();
    }

    fn into_window_size(width: u32, height: u32, font_size: f32) -> WindowSize {
        let num_lines = (height / (font_size as u32)) as u16;
        let num_cols = (width / (font_size as u32)) as u16;
        WindowSize {
            num_lines,
            num_cols,
            cell_width: 8,
            cell_height: 8,
        }
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
