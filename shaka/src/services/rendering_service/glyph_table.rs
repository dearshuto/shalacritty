use std::collections::HashMap;

pub struct Rect {
    pub offsetx: u32,
    pub offsety: u32,
    pub width: u32,
    pub height: u32,
}

pub struct GlyphTable {
    size: [u32; 2],

    table: HashMap<char, Rect>,
}

impl GlyphTable {
    pub fn new(width: u32, height: u32) -> Self {
        GlyphTable {
            size: [width, height],
            table: HashMap::new(),
        }
    }

    pub fn insert_glyph(
        &mut self,
        char: char,
        offsetx: u32,
        offsety: u32,
        width: u32,
        height: u32,
    ) {
        self.table.insert(
            char,
            Rect {
                offsetx,
                offsety,
                width,
                height,
            },
        );
    }

    // offsetx, offsety, width, height
    pub fn get_rect(&self, code: char) -> Option<&Rect> {
        let Some(rect) = self.table.get(&code) else {
            return None;
        };

        if !(rect.width > 0 && rect.height > 0) {
            return None;
        }

        Some(rect)
    }

    pub fn get_range(&self, code: char) -> Option<[f32; 4]> {
        let Some(rect) = self.table.get(&code) else {
            return None;
        };
        Some([
            rect.offsetx as f32 / self.size[0] as f32,
            rect.offsety as f32 / self.size[1] as f32,
            (rect.offsetx + rect.width) as f32 / self.size[0] as f32,
            (rect.offsety + rect.height) as f32 / self.size[1] as f32,
        ])
    }
}
