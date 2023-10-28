mod app;
mod config;
mod gfx;
mod tty;
mod window;

pub use app::App;
pub use config::{Config, ConfigService};
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
        VirtualKeyCode::Key0 => "0",
        VirtualKeyCode::Key1 => "1",
        VirtualKeyCode::Key2 => "2",
        VirtualKeyCode::Key3 => "3",
        VirtualKeyCode::Key4 => "4",
        VirtualKeyCode::Key5 => "5",
        VirtualKeyCode::Key6 => "6",
        VirtualKeyCode::Key7 => "7",
        VirtualKeyCode::Key8 => "8",
        VirtualKeyCode::Key9 => "9",
        VirtualKeyCode::Slash => return "/",
        VirtualKeyCode::Period => return ".",
        VirtualKeyCode::Back => return "\x7f",
        VirtualKeyCode::Space => return " ",
        VirtualKeyCode::Tab => return "\t",
        #[cfg(target_os = "macos")]
        VirtualKeyCode::Return => "\r",
        #[cfg(target_os = "linux")]
        VirtualKeyCode::Return => "\n",
        VirtualKeyCode::Up => "\x1b\x5b\x41",
        VirtualKeyCode::Down => "\x1b\x5b\x42",
        VirtualKeyCode::Right => "\x1b\x5b\x43",
        VirtualKeyCode::Left => "\x1b\x5b\x44",
        _ => return "",
    }
}
