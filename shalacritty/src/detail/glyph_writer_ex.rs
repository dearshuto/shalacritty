use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CoordRange {
    pub top_left: [f32; 2],
    pub bottom_right: [f32; 2],
}

pub struct GlyphWriterEx {
    image_width: u32,
    image_height: u32,

    block_width: u8,
    block_height: u8,

    coord: HashMap<char, CoordRange>,
}

impl GlyphWriterEx {
    pub fn new(block_width: u8, block_height: u8) -> Self {
        Self {
            image_width: 4096,
            image_height: 4096,
            block_width,
            block_height,
            coord: HashMap::default(),
        }
    }

    pub fn allocate(&mut self, code: char) -> bool {
        let stride_x = self.image_width / self.block_width as u32;
        let len = self.coord.len();
        let x = len % stride_x as usize;
        let y = len / stride_x as usize;
        let top_left = nalgebra::Vector2::new(
            (self.block_width as usize * x) as f32,
            (self.image_height as usize * y) as f32,
        );
        let bottom_right =
            top_left + nalgebra::Vector2::new(self.block_width as f32, self.block_height as f32);

        let matrix = nalgebra::Matrix3::new_nonuniform_scaling(&nalgebra::Vector2::new(
            (self.image_width as f32).recip(),
            (self.image_height as f32).recip(),
        ));
        let top_left_coord = matrix.transform_vector(&top_left);
        let bottom_right_coord = matrix.transform_vector(&bottom_right);

        self.coord.insert(
            code,
            CoordRange {
                top_left: top_left_coord.into(),
                bottom_right: bottom_right_coord.into(),
            },
        );

        true
    }

    pub fn get_coord_range(&self, code: char) -> Option<CoordRange> {
        let Some(coord) = self.coord.get(&code) else {
            return None;
        };

        Some(coord.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::GlyphWriterEx;

    #[test]
    fn allocate_simple() {
        let mut writer = GlyphWriterEx::new(8, 8);
        assert!(writer.allocate('a'));

        assert!(writer.get_coord_range('a').is_some());
        assert!(writer.get_coord_range('n').is_none());
    }

    #[test]
    fn allocate_multi() {
        let mut writer = GlyphWriterEx::new(8, 8);
        for code in 'a'..'z' {
            assert!(writer.allocate(code));
        }

        let stride_x = 0.001953125;
        let stride_y = 0.001953125;

        let range_a = writer.get_coord_range('a').unwrap();
        assert_eq!(range_a.top_left, [0.0, 0.0]);
        assert_eq!(range_a.bottom_right, [stride_x, stride_y]);

        let range_b = writer.get_coord_range('b').unwrap();
        assert_eq!(range_b.top_left, [stride_x, 0.0]);
        assert_eq!(
            range_b.bottom_right,
            [
                range_b.top_left[0] + stride_x,
                range_b.top_left[1] + stride_y
            ]
        );
    }
}
