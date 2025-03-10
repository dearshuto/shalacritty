use std::{
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use super::Config;

pub fn load_config<TPath>(path: TPath) -> Config
where
    TPath: AsRef<Path>,
{
    let mut file = std::fs::File::open(path.as_ref()).unwrap();
    let mut str: String = String::new();
    file.read_to_string(&mut str).ok().unwrap();
    let mut config: Config = toml::from_str(&str).unwrap();

    // 画像パスは設定ファイルからの相対パスにする
    let mut image_path = path.as_ref().to_path_buf();
    image_path.pop();
    image_path.push(config.image);
    config.image = image_path.to_str().unwrap().to_string();

    for path in &mut config.background.path {
        image_path.push(&path);
        *path = image_path.to_str().unwrap().to_string();
        image_path.pop();
    }

    config
}

pub fn create_config_directory() -> PathBuf {
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
