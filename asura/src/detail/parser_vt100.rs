use crate::parser::{Cell, TerminalParser};

pub struct ParserVt100 {
    parser: vt100::Parser,
}

impl ParserVt100 {
    pub fn new() -> Self {
        Self {
            parser: vt100::Parser::new(24, 80, 80),
        }
    }
}

impl TerminalParser for ParserVt100 {
    fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    fn set_size(&mut self, size: crate::types::TerminalSize) {
        self.parser.set_size(size.rows, size.cols);
    }

    fn cell(&self, row: u16, col: u16) -> Option<crate::parser::Cell> {
        let Some(cell) = self.parser.screen().cell(row, col) else {
            return None;
        };

        if !cell.has_contents() {
            return None;
        }

        Some(Cell {
            ch: cell.contents().chars().next().unwrap(),
        })
    }

    fn cursor_position(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }
}
