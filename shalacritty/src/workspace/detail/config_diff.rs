use crate::Config;

pub struct ConfigDiff {
    old_config: Config,

    font_size: Option<f32>,

    clear_color: Option<[f32; 4]>,

    image_alpha: Option<f32>,

    image_path_tentative: Option<String>,

    background_image_path: Option<Vec<String>>,
}

impl ConfigDiff {
    pub fn new() -> Self {
        Self {
            old_config: Config::default(),
            font_size: None,
            clear_color: None,
            image_alpha: None,
            image_path_tentative: None,
            background_image_path: None,
        }
    }

    pub fn update(&mut self, config: &Config) {
        // フォントサイズ
        if config.font_size != self.old_config.font_size {
            self.font_size = Some(config.font_size);
            self.old_config.font_size = config.font_size;
        }

        // 背景色
        if config.background.clear_color != self.old_config.background.clear_color {
            self.clear_color = Some(config.background.clear_color);
            self.old_config.background.clear_color = config.background.clear_color;
        } else {
            self.clear_color = None;
        }

        // 画像の透過度
        if config.image_alpha != self.old_config.image_alpha {
            self.image_alpha = Some(config.image_alpha);
            self.old_config.image_alpha = config.image_alpha;
        } else {
            self.image_alpha = None;
        }

        // 画像パス
        if config.image != self.old_config.image {
            self.image_path_tentative = Some(config.image.clone());
            self.old_config.image.clone_from(&config.image);
        } else {
            self.image_path_tentative = None;
        }
    }

    pub fn is_dirty(&self) -> bool {
        if self.clear_color.is_some() {
            return true;
        }

        if self.image_alpha.is_some() {
            return true;
        }

        if self.image_path_tentative.is_some() {
            return true;
        }

        false
    }

    pub fn consume_clear_color(&mut self) -> Option<[f32; 4]> {
        let mut dst = None;
        std::mem::swap(&mut dst, &mut self.clear_color);
        dst
    }

    #[allow(dead_code)]
    pub fn consume_background_path(&mut self) -> Option<Vec<String>> {
        let mut dst = None;
        std::mem::swap(&mut dst, &mut self.background_image_path);
        dst
    }

    pub fn consume_background_path_migrated(&mut self) -> Option<String> {
        let mut dst = None;
        std::mem::swap(&mut dst, &mut self.image_path_tentative);
        dst
    }
}
