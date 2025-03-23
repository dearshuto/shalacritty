use std::{collections::HashMap, sync::mpsc::TryRecvError};

use asura::TerminalEmulator;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

use crate::{
    app::{KeyboadInputEventArgs, WindowCreatedEventArgs, WindowSizeChangedEventArgs},
    workspace::Action,
    Config,
};

pub struct ShellService {
    multiplexer: asura::Multiplexer,
    terminal_emulator: asura::TerminalEmulator,

    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_created_receiver: std::sync::mpsc::Receiver<WindowCreatedEventArgs>,
    receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    input_receiver: std::sync::mpsc::Receiver<KeyboadInputEventArgs>,
    polling_event_receiver: tokio::sync::mpsc::Receiver<()>,

    string_senders: Vec<tokio::sync::mpsc::Sender<String>>,

    active_shell_id: Option<asura::ShellId>,
    shell_controller_table: HashMap<asura::ShellId, asura::ShellController>,

    font_size: Option<f32>,
    window_width: Option<u32>,
    window_height: Option<u32>,
}

impl ShellService {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        window_created_receiver: std::sync::mpsc::Receiver<WindowCreatedEventArgs>,
        receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        input_receiver: std::sync::mpsc::Receiver<KeyboadInputEventArgs>,
        polling_event_receiver: tokio::sync::mpsc::Receiver<()>,
    ) -> Self {
        let (_id, terminal_emulator) = TerminalEmulator::new();

        Self {
            multiplexer: asura::Multiplexer::new(),
            terminal_emulator,
            window_created_receiver,
            config_receiver,
            receiver,
            input_receiver,
            polling_event_receiver,
            string_senders: Vec::default(),
            active_shell_id: None,
            shell_controller_table: HashMap::default(),
            font_size: None,
            window_width: None,
            window_height: None,
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(config) = self.config_receiver.recv() => self.apply_config(config),
            Some(args) = self.receiver.recv() => self.apply_window_size(args),
            Some(_) = self.polling_event_receiver.recv() => self.try_estimate_teletype_events().await,
            else => break,
            );
        }
    }

    pub fn listen_string(&mut self) -> tokio::sync::mpsc::Receiver<String> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.string_senders.push(sender);
        receiver
    }

    fn apply_config(&mut self, config: Config) {
        // フォントサイズに変更がない場合は何もしない
        if let Some(font_size) = self.font_size {
            if font_size == config.font_size {
                return;
            }
        }

        self.font_size = Some(config.font_size);

        let Some(window_width) = self.window_width else {
            return;
        };

        let Some(window_height) = self.window_height else {
            return;
        };

        self.terminal_emulator.resize(window_width, window_height);
    }

    fn apply_window_size(&mut self, args: WindowSizeChangedEventArgs) {
        self.window_width = Some(args.width);
        self.window_height = Some(args.height);

        self.terminal_emulator.resize(args.width, args.height);
    }

    async fn try_estimate_teletype_events(&mut self) {
        // ウィンドウが作成されたらそこにシェルを割り当てる
        // とりあえずウィンドウは単一であることを仮定する
        if let Ok(_args) = self.window_created_receiver.try_recv() {
            let (shell_id, controller) = self.multiplexer.spawn(&asura::Config::default());
            self.active_shell_id = Some(shell_id);
            self.shell_controller_table.insert(shell_id, controller);
        }

        // 終了していたシェルを辞書から除外する
        let mut dirty_shell_ids = Vec::new();
        self.shell_controller_table.retain(|key, controller| {
            match controller.try_recv_event() {
                Ok(_) => {
                    // なにか起きたシェルに再描画
                    dirty_shell_ids.push(*key);
                    true
                }
                Err(error) => match error {
                    TryRecvError::Empty => true,
                    TryRecvError::Disconnected => false,
                },
            }
        });

        // 暫定実装
        // 再描画処理
        let tasks: Vec<_> = dirty_shell_ids
            .into_iter()
            .filter_map(|id| {
                let controller = self.shell_controller_table.get(&id)?;

                let contents: String = controller
                    .read_contents()
                    .acquire_contents()
                    .into_iter()
                    .map(|x| x.code)
                    .collect();

                let mut task = Vec::default();
                for sender in &self.string_senders {
                    let handle = sender.send(contents.clone());
                    task.push(handle);
                }
                Some(task)
            })
            .flatten()
            .collect();
        futures::future::join_all(tasks).await;

        if let Ok(args) = self.input_receiver.try_recv() {
            self.apply_input(args).await;
        }
    }

    async fn apply_input(&mut self, args: KeyboadInputEventArgs) {
        let Some(active_shell_id) = self.active_shell_id else {
            return;
        };

        let Some(controller) = self.shell_controller_table.get_mut(&active_shell_id) else {
            return;
        };

        let text_with_all_modifiers = args.event.text_with_all_modifiers().unwrap_or_default();
        let action = crate::app::detect_action(text_with_all_modifiers, args.state);
        match action {
            Action::Input(text) => controller.send_input(text),
            Action::Paste => todo!(),
            Action::SplitHorizontal => todo!(),
            Action::NewTab => todo!(),
            Action::ActivateTab(_) => todo!(),
            Action::ActivateNextTile => todo!(),
            Action::DumpDebugInfo => todo!(),
        }
    }
}
