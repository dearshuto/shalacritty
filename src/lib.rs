mod app;
mod config;
mod gfx;
mod tty;
mod window;
mod workspace;

pub use app::App;
pub use config::{Config, ConfigService};
use winit::event::KeyEvent;

// static ss: String = b'0x7f'.to_string();

pub fn convert_key_to_str(key_event: KeyEvent) -> String {
    let Some(text) = &key_event.text else {
        return "".to_string();
    };

    text.as_str().to_string()
}
