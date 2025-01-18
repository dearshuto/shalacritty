use std::{collections::HashMap, str::FromStr, time::Duration};

use color_eyre::{eyre::Ok, Result};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    widgets::{Block, Borders, List, ListItem, Widget},
    DefaultTerminal,
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}

struct App {
    #[allow(dead_code)]
    virtual_window_manager: vw::VirtualWindowManager,

    #[allow(dead_code)]
    multiplexer: asura::Multiplexer,

    window_shell_table: HashMap<vw::VirtualWindowId, asura::ShellId>,
}

impl Drop for App {
    fn drop(&mut self) {}
}

impl App {
    pub fn new() -> Self {
        let virtual_window_manager = vw::VirtualWindowManager::new();
        let virtual_window_id = virtual_window_manager.ids()[0];

        let mut multiplexer = asura::Multiplexer::new();
        let shell_id = multiplexer.spawn(640, 480);

        let mut shell_event = multiplexer.subscribe_shell_event(shell_id);
        multiplexer.input(shell_id, String::from_str("pwd\n").unwrap().as_bytes());
        shell_event.wait_signaled();

        Self {
            virtual_window_manager,
            multiplexer,
            window_shell_table: HashMap::from([(virtual_window_id, shell_id)]),
        }
    }

    /// Run the app until the user exits.
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let duration = Duration::from_millis(16);
        loop {
            terminal.draw(|frame| {
                // 描画領域を更新
                self.virtual_window_manager
                    .resize_root(frame.area().width as u32, frame.area().height as u32);

                // 描画領域をシェルに反映
                for (virtual_window_id, shell_id) in &self.window_shell_table {
                    let Some(virtual_window) = self
                        .virtual_window_manager
                        .try_get_virtual_window(*virtual_window_id)
                    else {
                        continue;
                    };

                    self.multiplexer.resize(
                        *shell_id,
                        virtual_window.width(),
                        virtual_window.height(),
                    );
                }

                frame.render_widget(&self, frame.area())
            })?;
            while event::poll(duration)? {
                let Event::Key(key) = event::read()? else {
                    continue;
                };

                match key.kind {
                    KeyEventKind::Press => {
                        let id = self.window_shell_table.values().next().unwrap();
                        let code = key.code;
                        let bytes: Vec<u8> = match code {
                            event::KeyCode::Backspace => asura::util::Unicode::backspace().to_vec(),
                            event::KeyCode::Enter => asura::util::Unicode::enter().to_vec(),
                            // event::KeyCode::Left => todo!(),
                            // event::KeyCode::Right => todo!(),
                            // event::KeyCode::Up => todo!(),
                            // event::KeyCode::Down => todo!(),
                            // event::KeyCode::Home => todo!(),
                            // event::KeyCode::End => todo!(),
                            // event::KeyCode::PageUp => todo!(),
                            // event::KeyCode::PageDown => todo!(),
                            // event::KeyCode::Tab => todo!(),
                            // event::KeyCode::BackTab => todo!(),
                            // event::KeyCode::Delete => todo!(),
                            // event::KeyCode::Insert => todo!(),
                            // event::KeyCode::F(_) => todo!(),
                            event::KeyCode::Char(c) => c.to_string().as_bytes().to_vec(),
                            // event::KeyCode::Null => todo!(),
                            event::KeyCode::Esc => return Ok(()),
                            // event::KeyCode::CapsLock => todo!(),
                            // event::KeyCode::ScrollLock => todo!(),
                            // event::KeyCode::NumLock => todo!(),
                            // event::KeyCode::PrintScreen => todo!(),
                            // event::KeyCode::Pause => todo!(),
                            // event::KeyCode::Menu => todo!(),
                            // event::KeyCode::KeypadBegin => todo!(),
                            // event::KeyCode::Media(media_key_code) => todo!(),
                            // event::KeyCode::Modifier(modifier_key_code) => todo!(),
                            _ => vec![],
                        };
                        self.multiplexer.input(*id, &bytes);
                    }
                    KeyEventKind::Repeat => todo!(),
                    KeyEventKind::Release => todo!(),
                }
            }
        }
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // 表示要素を ratatui のオブジェクトに変換
        let id = self.window_shell_table.values().next().unwrap();
        let content = self.multiplexer.enumerate_content(*id).unwrap();
        let messages = content.lines().map(|str| ListItem::new(str));

        List::new(messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{}x{}", area.width, area.height)),
            )
            .render(area, buf);
    }
}
