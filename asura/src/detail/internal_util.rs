use alacritty_terminal::{term::color::Colors, vte::ansi::Color};

pub fn convert_color_uint(color: &Color, colors: &Colors) -> [u8; 3] {
    let rgb = match color {
        &alacritty_terminal::vte::ansi::Color::Named(named_color) => colors[named_color],
        &alacritty_terminal::vte::ansi::Color::Spec(rgb) => Some(rgb),
        &alacritty_terminal::vte::ansi::Color::Indexed(index) => colors[index as usize],
    }
    .unwrap_or_default();

    [rgb.r, rgb.g, rgb.b]
}

pub fn convert_color_snorm(color: &Color, colors: &Colors) -> [f32; 3] {
    let rgb = convert_color_uint(color, colors);
    into_snorm(&rgb)
}

pub fn into_snorm(rgb: &[u8; 3]) -> [f32; 3] {
    [
        rgb[0] as f32 / u8::MAX as f32,
        rgb[1] as f32 / u8::MAX as f32,
        rgb[2] as f32 / u8::MAX as f32,
    ]
}

#[cfg(test)]
mod tests {

    #[test]
    fn into_snorm() {
        assert_eq!([0.0, 0.0, 0.0], super::into_snorm(&[0, 0, 0]));
    }
}
