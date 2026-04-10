use std::collections::HashMap;

pub struct GlyphTable {
    size: [u32; 2],

    // offsetx, offsety, width, height
    table: HashMap<char, [u32; 4]>,
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
        self.table.insert(char, [offsetx, offsety, width, height]);
    }

    // offsetx, offsety, width, height
    pub fn get_rect(&self, code: char) -> Option<[u32; 4]> {
        let Some(rect) = self.table.get(&code) else {
            return None;
        };

        if !(rect[2] > 0 && rect[3] > 0) {
            return None;
        }

        Some(*rect)
    }

    pub fn get_range(&self, code: char) -> Option<[f32; 4]> {
        let Some([offsetx, offsety, width, height]) = self.table.get(&code) else {
            return None;
        };
        Some([
            *offsetx as f32 / self.size[0] as f32,
            *offsety as f32 / self.size[1] as f32,
            (*offsetx + *width) as f32 / self.size[0] as f32,
            (*offsety + *height) as f32 / self.size[1] as f32,
        ])
    }
}
