use std::{io::Read, path::PathBuf, str::FromStr};

use super::Config;

pub struct ConfigProxy;

impl ConfigProxy {
    pub fn new() -> Option<Config> {
        // 設定置き場はなければ作る
        let config_directory = Self::config_directory();
        if !config_directory.exists() {
            std::fs::create_dir(&config_directory).unwrap();
        }

        // 設定ファイルがなければ作る
        let config_path = {
            let mut config_path = config_directory.clone();
            config_path.push("config.toml");
            config_path
        };
        if !config_path.exists() {
            std::fs::File::create_new(&config_path);
        }

        let mut file = std::fs::File::open(&config_path).unwrap();
        let mut str: String = String::new();
        file.read_to_string(&mut str).ok().unwrap();
        Some(toml::from_str(&str).unwrap())
    }

    fn config_directory() -> PathBuf {
        std::env::home_dir();
        #[cfg(target_os = "windows")]
        let home_directory = std::env::var("APPDATA").unwrap();

        #[cfg(target_os = "macos")]
        let home_directory = std::env::var("HOME").unwrap();

        #[cfg(target_os = "linux")]
        let home_directory = std::env::var("HOME").unwrap();

        let mut config_directory_path = PathBuf::from_str(&home_directory).unwrap();
        config_directory_path.push(".config");
        config_directory_path.push("shalacritty");

        config_directory_path
    }
}
