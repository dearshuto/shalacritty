use alacritty_terminal::{
    index::{Column, Line, Point},
    vte::ansi::Color,
};

pub struct AsuraContentAdapter(asura::Content);

impl crate::gfx::IContent for AsuraContentAdapter {
    type TColor = Color;
    type TPosition = Point;

    fn code(&self) -> char {
        self.0.code
    }

    fn color_fg(&self) -> Self::TColor {
        let r = (self.0.fg[0] * 255.0).clamp(0.0, 255.0) as u8;
        let g = (self.0.fg[1] * 255.0).clamp(0.0, 255.0) as u8;
        let b = (self.0.fg[2] * 255.0).clamp(0.0, 255.0) as u8;
        Color::Spec(alacritty_terminal::vte::ansi::Rgb { r, g, b })
    }

    fn position(&self) -> Self::TPosition {
        let x = self.0.x;
        let y = self.0.y;
        Point {
            line: Line::from(y),
            column: Column::from(x),
        }
    }
}

impl From<asura::Content> for AsuraContentAdapter {
    fn from(value: asura::Content) -> Self {
        AsuraContentAdapter(value)
    }
}
