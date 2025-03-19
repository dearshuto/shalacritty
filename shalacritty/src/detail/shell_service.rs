use std::{collections::HashMap, sync::mpsc::TryRecvError};

use alacritty_terminal::{event::WindowSize, event_loop::EventLoopSender};
use asura::TeletypeId;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

use crate::{
    app::{KeyboadInputEventArgs, WindowCreatedEventArgs, WindowSizeChangedEventArgs},
    workspace::Action,
    Config,
};

use super::shell_util::TerminalParams;

pub struct ShellService {
    // シェル管理（載せ替え予定）
    #[allow(unused)]
    multiplexer: asura::Multiplexer,

    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_created_receiver: std::sync::mpsc::Receiver<WindowCreatedEventArgs>,
    receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    input_receiver: std::sync::mpsc::Receiver<KeyboadInputEventArgs>,
    polling_event_receiver: tokio::sync::mpsc::Receiver<()>,
    event_loop_sender_table: HashMap<TeletypeId, EventLoopSender>,

    event_receiver_table:
        HashMap<TeletypeId, std::sync::mpsc::Receiver<alacritty_terminal::event::Event>>,

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
        Self {
            multiplexer: asura::Multiplexer::new(),
            window_created_receiver,
            config_receiver,
            receiver,
            input_receiver,
            polling_event_receiver,
            event_loop_sender_table: HashMap::default(),
            event_receiver_table: HashMap::default(),
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

        let window_size = super::shell_util::calculate_terminal_window_size(&TerminalParams {
            font_size: config.font_size,
            line_spacing: 1.0, // TODO
            window_width,
            window_height,
        });

        for sender in self.event_loop_sender_table.values() {
            let msg = alacritty_terminal::event_loop::Msg::Resize(window_size);
            sender.send(msg).unwrap();
        }
    }

    fn apply_window_size(&mut self, args: WindowSizeChangedEventArgs) {
        let Some(font_size) = self.font_size else {
            return;
        };

        self.window_width = Some(args.width);
        self.window_height = Some(args.height);

        let window_size = super::shell_util::calculate_terminal_window_size(&TerminalParams {
            font_size,
            line_spacing: 1.0, // TODO
            window_width: args.width,
            window_height: args.height,
        });

        for sender in self.event_loop_sender_table.values() {
            let msg = alacritty_terminal::event_loop::Msg::Resize(window_size);
            sender.send(msg).unwrap();
        }
    }

    async fn try_estimate_teletype_events(&mut self) {
        // ウィンドウが作成されたらそこにシェルを割り当てる
        // とりあえずウィンドウは単一であることを仮定する
        if let Ok(_args) = self.window_created_receiver.try_recv() {
            let (shell_id, controller) = self.multiplexer.spawn(&asura::Config::default());
            self.active_shell_id = Some(shell_id);
            self.shell_controller_table.insert(shell_id, controller);
        }

        self.event_receiver_table
            .retain(|_id, receiver| match receiver.try_recv() {
                Ok(_event) => {
                    // シェルの内容に変更があった
                    // TODO: ここでイベントを通知する
                    return true;
                }
                Err(error) => {
                    match error {
                        // シェルの内容に変更はなかったのでなにもしない
                        TryRecvError::Empty => return true,
                        // 変更元が破棄されていたらもう購読する意味がないので receiver を破棄
                        TryRecvError::Disconnected => return false,
                    };
                }
            });

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

        let contents: String = controller
            .read_contents()
            .acquire_contents()
            .into_iter()
            .map(|x| x.code)
            .collect();
        for sender in &self.string_senders {
            sender.send(contents.clone()).await.unwrap();
        }
    }
}
