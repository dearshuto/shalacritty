pub struct Unicode;

impl Unicode {
    pub fn allow_up() -> &'static [u8] {
        &[0x1b, 0x5b, 0x41]
    }

    pub fn allow_down() -> &'static [u8] {
        &[0x1b, 0x5b, 0x42]
    }

    pub fn allow_right() -> &'static [u8] {
        &[0x1b, 0x5b, 0x43]
    }

    pub fn allow_left() -> &'static [u8] {
        &[0x1b, 0x5b, 0x44]
    }

    pub fn backspace() -> &'static [u8] {
        &[0x0008]
    }

    pub fn enter() -> &'static [u8] {
        "\n".as_bytes()
    }
}
