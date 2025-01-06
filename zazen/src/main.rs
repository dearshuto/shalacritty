use std::time::Duration;

use color_eyre::{eyre::Ok, Result};
use crossterm::event::{self, Event};
use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Span},
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

struct App;

impl App {
    pub fn new() -> Self {
        Self {}
    }

    /// Run the app until the user exits.
    fn run(self, mut terminal: DefaultTerminal) -> Result<()> {
        let duration = Duration::from_millis(16);
        loop {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            while event::poll(duration)? {
                if let Event::Key(_key) = event::read()? {
                    return Ok(());
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
        let areas = Layout::vertical([Constraint::Ratio(1, 2); 2]).split(area);

        let messages: Vec<ListItem> = vec![
            ListItem::new(Line::from(Span::raw("Hello First World!"))),
            ListItem::new(Line::from(Span::raw("Hello Second World!"))),
        ];
        List::new(messages)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "First Area: {}x{}",
                areas[0].width, areas[0].height
            )))
            .render(areas[0], buf);

        let messages = vec![ListItem::new(Line::from(Span::raw("Hello New World!")))];
        List::new(messages)
            .block(Block::default().borders(Borders::ALL).title("Second Area"))
            .render(areas[1], buf);
    }
}
