use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct KeyEventBridge(KeyEvent);

impl KeyEventBridge {
    pub fn new(event: KeyEvent) -> Self {
        Self(event)
    }
}

impl KeyEventBridge {
    pub fn convert(self) -> Result<String, std::io::Error> {
        let key = self.0;
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char(c) = key.code {
                let control_byte = if c.is_ascii_lowercase() {
                    c as u8 - b'a' + 1
                } else if c.is_ascii_uppercase() {
                    c as u8 - b'A' + 1
                } else if c == ' ' {
                    0x00 // Ctrl+Space is Null (0x00)
                } else {
                    c as u8 // Fallback for other characters, might not be ideal
                };
                return Ok(String::from_utf8(vec![control_byte]).unwrap_or_default());
            } else {
                return Ok(String::new());
            }
        }

        match key.code {
            KeyCode::Backspace => {
                Ok(String::from_utf8(asura::util::Unicode::backspace().to_vec()).unwrap())
            }
            KeyCode::Enter => {
                Ok(String::from_utf8(asura::util::Unicode::enter().to_vec()).unwrap())
            }
            KeyCode::Left => {
                Ok(String::from_utf8(asura::util::Unicode::allow_left().to_vec()).unwrap())
            }
            KeyCode::Right => {
                Ok(String::from_utf8(asura::util::Unicode::allow_right().to_vec()).unwrap())
            }
            KeyCode::Up => {
                Ok(String::from_utf8(asura::util::Unicode::allow_up().to_vec()).unwrap())
            }
            KeyCode::Down => {
                Ok(String::from_utf8(asura::util::Unicode::allow_down().to_vec()).unwrap())
            }
            // event::KeyCode::Home => todo!(),
            // event::KeyCode::End => todo!(),
            // event::KeyCode::PageUp => todo!(),
            // event::KeyCode::PageDown => todo!(),
            // event::KeyCode::Tab => todo!(),
            // event::KeyCode::BackTab => todo!(),
            // event::KeyCode::Delete => todo!(),
            // event::KeyCode::Insert => todo!(),
            // event::KeyCode::F(_) => todo!(),
            KeyCode::Char(c) => Ok(c.to_string()),
            // event::KeyCode::Null => todo!(),
            KeyCode::Esc => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Esc")),
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
    }
}
