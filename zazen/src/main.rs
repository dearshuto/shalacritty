use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use color_eyre::{eyre::Ok, Result};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
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

                frame.render_widget(&self, frame.area())
            })?;
            while event::poll(duration)? {
                match event::read()? {
                    Event::Key(key) => match key.kind {
                        KeyEventKind::Press => {
                            let controller = self.controller_table.values_mut().next().unwrap();
                            let str = if key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                if let event::KeyCode::Char(c) = key.code {
                                    let control_byte = if c.is_ascii_lowercase() {
                                        c as u8 - b'a' + 1
                                    } else if c.is_ascii_uppercase() {
                                        c as u8 - b'A' + 1
                                    } else if c == ' ' {
                                        0x00 // Ctrl+Space is Null (0x00)
                                    } else {
                                        c as u8 // Fallback for other characters, might not be ideal
                                    };
                                    Ok(String::from_utf8(vec![control_byte]).unwrap_or_default())
                                } else {
                                    Ok(String::new())
                                }
                            } else {
                                match key.code {
                                    event::KeyCode::Backspace => Ok(String::from_utf8(
                                        asura::util::Unicode::backspace().to_vec(),
                                    )
                                    .unwrap()),
                                    event::KeyCode::Enter => {
                                        Ok(String::from_utf8(asura::util::Unicode::enter().to_vec())
                                            .unwrap())
                                    }
                                    event::KeyCode::Left => Ok(String::from_utf8(asura::util::Unicode::allow_left().to_vec()).unwrap()),
                                    event::KeyCode::Right => Ok(String::from_utf8(asura::util::Unicode::allow_right().to_vec()).unwrap()),
                                    event::KeyCode::Up => Ok(String::from_utf8(asura::util::Unicode::allow_up().to_vec()).unwrap()),
                                    event::KeyCode::Down => Ok(String::from_utf8(asura::util::Unicode::allow_down().to_vec()).unwrap()),
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
                                }
                            };
                            controller.send_input(&str.unwrap());
                        }
                        KeyEventKind::Repeat => todo!(),
                        KeyEventKind::Release => todo!(),
                    },
                    Event::Resize(new_width, new_height) => {
                        let controller = self.controller_table.values_mut().next().unwrap();
                        controller.resize(new_height, new_width, 1, 1);
                    }
                    _ => {}
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
                |mut tree: BTreeMap<i32, Vec<asura::Content>>, value| {
                    tree.entry(value.y).or_insert_with(Vec::new).push(value.clone());
                    tree
                },
            )
            .into_iter()
            .map(|(_key, contents)| {
                let spans: Vec<Span> = contents
                    .into_iter()
                    .map(|content| {
                        let r = (content.fg[0] * 255.0) as u8;
                        let g = (content.fg[1] * 255.0) as u8;
                        let b = (content.fg[2] * 255.0) as u8;
                        Span::styled(
                            content.code.to_string(),
                            Style::default().fg(Color::Rgb(r, g, b)),
                        )
                    })
                    .collect();
                ListItem::new(Text::from(Line::from(spans)))
            });

        List::new(messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("{}x{}", area.width, area.height)),
            )
            .render(area, buf);

        // Draw cursor
        let cursor_point = terminal_accessor.get_cursor_point();
        let cursor_x = cursor_point.0 as u16; // Corrected access for x
        let cursor_y = cursor_point.1 as u16; // Corrected access for y

        // Ensure cursor is within the visible area and within the drawing area bounds
        if cursor_x < area.width && cursor_y < area.height {
            // Get the cell at the cursor position
            let cell = buf.cell_mut((area.x + cursor_x, area.y + cursor_y)).unwrap(); // Corrected method call and unwrapped Option
            // Set its background color to indicate the cursor
            // For a "2px wide bar", in a character-based terminal, we highlight the cell
            // as this is the closest visual representation possible.
            cell.set_bg(Color::LightGreen);
        }
    }
}
