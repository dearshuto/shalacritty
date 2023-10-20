use alacritty_terminal::term::RenderableContent;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct CharacterInfo {
    pub code: char,
    // pub transform: nalgebra::Matrix3x2<f32>,
    // pub uv0: nalgebra::Vector2<f32>,
    // pub uv1: nalgebra::Vector2<f32>,
}

pub struct ContentPlotter {
    old_items: Vec<CharacterInfo>,
}

impl ContentPlotter {
    pub fn new() -> Self {
        Self {
            old_items: Vec::default(),
        }
    }

    pub fn calculate_diff(&mut self, _renderable_content: RenderableContent) -> Vec<CharacterInfo> {
        let items = vec![CharacterInfo {
            code: 'F',
            // transform: nalgebra::Matrix3x2::identity(),
            // uv0: Vector2::default(),
            // uv1: Vector2::default(),
        }];

        if self.old_items == items {
            return vec![];
        }

        self.old_items = items.clone();
        return items;
    }
}
