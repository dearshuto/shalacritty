use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

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
    multiplexer: asura::Multiplexer,

    controller_table: HashMap<asura::ShellId, asura::ShellController>,
}

impl Drop for App {
    fn drop(&mut self) {}
}

impl App {
    pub fn new() -> Self {
        let mut multiplexer = asura::Multiplexer::new();
        let key_value = multiplexer.spawn(&asura::Config::default());

        Self {
            multiplexer,
            controller_table: HashMap::from([key_value]),
        }
    }

    /// Run the app until the user exits.
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let duration = Duration::from_millis(16);
        loop {
            terminal.draw(|frame| {
                // TODO: 描画領域をシェルに反映する
                frame.render_widget(&self, frame.area())
            })?;
            while event::poll(duration)? {
                let Event::Key(key) = event::read()? else {
                    continue;
                };

                match key.kind {
                    KeyEventKind::Press => {
                        let controller = self.controller_table.values_mut().next().unwrap();
                        let code = key.code;
                        let str = match code {
                            event::KeyCode::Backspace => Ok(String::from_utf8(
                                asura::util::Unicode::backspace().to_vec(),
                            )
                            .unwrap()),
                            event::KeyCode::Enter => {
                                Ok(String::from_utf8(asura::util::Unicode::enter().to_vec())
                                    .unwrap())
                            }
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
                            event::KeyCode::Char(c) => Ok(c.to_string()),
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
                            _ => Ok(String::new()),
                        };

                        controller.send_input(&str.unwrap());
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

        let controller: &asura::ShellController = self.controller_table.values().next().unwrap();
        let terminal_accessor = controller.read_contents();

        // Y 座標でグループ化
        // 各グループを 1 行の表示単位として ListItem に変換していく
        let messages = terminal_accessor
            .acquire_contents()
            .iter()
            .fold(
                BTreeMap::default(),
                |mut tree: BTreeMap<i32, String>, value| {
                    tree.entry(value.y)
                        .or_insert_with(String::new)
                        .push(value.code);

                    tree
                },
            )
            .into_iter()
            .map(|(_key, value)| ListItem::new(value));

        List::new(messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{}x{}", area.width, area.height)),
            )
            .render(area, buf);
    }
}
