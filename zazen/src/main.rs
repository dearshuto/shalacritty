use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal, 
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Widget},
};

mod tui;

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = tui::init()?;
    let app_result = App::new().run(&mut terminal);
    tui::restore()?;
    app_result
}

struct App {
    terminal_emulator: asura::TerminalEmulator,
    controller_table: HashSet<asura::ShellId>,
}

impl Drop for App {
    fn drop(&mut self) {}
}

impl App {
    pub fn new() -> Self {
        asura::TerminalEmulator::new();
        let (_tab_id, shell_id, terminal_emulator) = asura::TerminalEmulator::new();

        Self {
            terminal_emulator,
            controller_table: HashSet::from([shell_id]),
        }
    }

    /// Run the app until the user exits.
    fn run(mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        let duration = Duration::from_millis(16);

        loop {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            while event::poll(duration)? {
                match event::read()? {
                    Event::Key(key) => match key.kind {
                        KeyEventKind::Press => {
                            let Ok(str) = zazen::KeyEventBridge::new(key).convert() else {
                                return Ok(());
                            };
                            let shell_id = self.controller_table.iter().next().unwrap();
                            self.terminal_emulator.send_input(*shell_id, &str);
                        }
                        KeyEventKind::Repeat => todo!(),
                        KeyEventKind::Release => todo!(),
                    },
                    Event::Resize(new_width, new_height) => {
                        self.terminal_emulator
                            .resize(new_width as u32, new_height as u32);
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

        let shell_id = self.controller_table.iter().next().unwrap();
        self.terminal_emulator
            .acquire_content(*shell_id, |contents, (cursor_x, cursor_y)| {
                //
                // Y 座標でグループ化
                // 各グループを 1 行の表示単位として ListItem に変換していく
                let messages = contents
                    .iter()
                    .fold(
                        BTreeMap::default(),
                        |mut tree: BTreeMap<i32, Vec<asura::Content>>, value| {
                            tree.entry(value.y)
                                .or_insert_with(Vec::new)
                                .push(value.clone());
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

                let cursor_x = cursor_x as u16;
                let cursor_y = cursor_y as u16;

                // Ensure cursor is within the visible area and within the drawing area bounds
                if cursor_x < area.width && cursor_y < area.height {
                    // Get the cell at the cursor position
                    let cell = buf
                        .cell_mut((area.x + cursor_x, area.y + cursor_y))
                        .unwrap(); // Corrected method call and unwrapped Option
                                   // Set its background color to indicate the cursor
                                   // For a "2px wide bar", in a character-based terminal, we highlight the cell
                                   // as this is the closest visual representation possible.
                    cell.set_bg(Color::LightGreen);
                }
            })
    }
}
