use alacritty_terminal::{
    grid::Indexed,
    index::{Column, Line, Point},
    term::cell::Cell,
    vte::ansi::Color,
};

pub struct PositionAdapter {
    point: Point,
}

impl crate::multiplexers::IPosition for PositionAdapter {
    fn new(x: u32, y: u32) -> Self {
        Self {
            point: Point {
                column: Column::from(x as usize),
                line: Line::from(y as usize),
            },
        }
    }
}

pub struct ContentAdapter {
    cell: Indexed<Cell>,
}

impl crate::gfx::IContent for ContentAdapter {
    type TColor = Color;
    type TPosition = Point;

    fn code(&self) -> char {
        self.cell.c
    }

    fn color_fg(&self) -> Self::TColor {
        self.cell.fg
    }

    fn position(&self) -> Self::TPosition {
        self.cell.point
    }
}

impl crate::multiplexers::IContent for ContentAdapter {
    type TPosition = PositionAdapter;
    fn with_offset(mut self, offset: &Self::TPosition) -> Self {
        self.cell.point.column += offset.point.column;
        self.cell.point.line += offset.point.line;
        self
    }
}

impl From<Indexed<&Cell>> for ContentAdapter {
    fn from(value: Indexed<&Cell>) -> Self {
        Self {
            cell: Indexed {
                point: value.point,
                cell: value.cell.clone(),
            },
        }
    }
}
