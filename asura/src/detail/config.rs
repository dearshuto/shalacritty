use alacritty_terminal::term::Config;

pub struct ConfigBridge;

impl ConfigBridge {
    pub fn generate_default() -> alacritty_terminal::term::Config {
        Config::default()
    }
}
