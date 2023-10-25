mod app;
mod gfx;
mod tty;
mod window;

pub use app::App;
use winit::event::VirtualKeyCode;

// static ss: String = b'0x7f'.to_string();

pub fn convert_key_to_str(key_code: VirtualKeyCode) -> &'static str {
    match key_code {
        VirtualKeyCode::A => return "a",
        VirtualKeyCode::B => return "b",
        VirtualKeyCode::C => return "c",
        VirtualKeyCode::D => return "d",
        VirtualKeyCode::E => return "e",
        VirtualKeyCode::F => return "f",
        VirtualKeyCode::G => return "g",
        VirtualKeyCode::H => return "h",
        VirtualKeyCode::I => return "i",
        VirtualKeyCode::J => return "j",
        VirtualKeyCode::K => return "k",
        VirtualKeyCode::L => return "l",
        VirtualKeyCode::M => return "m",
        VirtualKeyCode::N => return "n",
        VirtualKeyCode::O => return "o",
        VirtualKeyCode::P => return "p",
        VirtualKeyCode::Q => return "q",
        VirtualKeyCode::R => return "r",
        VirtualKeyCode::S => return "s",
        VirtualKeyCode::T => return "t",
        VirtualKeyCode::U => return "u",
        VirtualKeyCode::V => return "v",
        VirtualKeyCode::W => return "w",
        VirtualKeyCode::X => return "x",
        VirtualKeyCode::Y => return "y",
        VirtualKeyCode::Z => return "z",
        VirtualKeyCode::Slash => return "/",
        VirtualKeyCode::Period => return ".",
        VirtualKeyCode::Back => return "\x7f",
        VirtualKeyCode::Space => return " ",
        VirtualKeyCode::Tab => return "\t",
        VirtualKeyCode::Return => "\n",
        _ => return "",
    }
}
