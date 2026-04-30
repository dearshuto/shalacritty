pub struct Section {
    pub offset: usize,
    pub size: usize,
}

pub struct BufferLayout {
    sections: Vec<Section>,
}

impl BufferLayout {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    /// Add a new section for type T with the specified count and alignment requirement.
    /// `alignment` can be any positive integer (not restricted to powers of two).
    pub fn add<T>(&mut self, count: usize, alignment: usize) -> usize {
        let index = self.sections.len();
        let previous_end = self.sections.last().map_or(0, |s| s.offset + s.size);

        // Robust alignment calculation for non-power-of-two values
        let aligned_offset = if alignment <= 1 {
            previous_end
        } else {
            let remainder = previous_end % alignment;
            if remainder == 0 {
                previous_end
            } else {
                previous_end + (alignment - remainder)
            }
        };

        self.sections.push(Section {
            offset: aligned_offset,
            size: std::mem::size_of::<T>() * count,
        });
        index
    }

    pub fn resize<T>(&mut self, index: usize, count: usize) {
        let section = &mut self.sections[index];
        section.size = std::mem::size_of::<T>() * count;
    }

    pub fn get_slice_mut<T>(&self, ptr: *mut T, index: usize) -> &mut [T] {
        let section = &self.sections[index];
        unsafe {
            // Correctly apply byte offset to the base pointer before casting
            let section_ptr = (ptr as *mut u8).add(section.offset) as *mut T;
            std::slice::from_raw_parts_mut(section_ptr, section.size / std::mem::size_of::<T>())
        }
    }

    pub fn get_section(&self, index: usize) -> &Section {
        &self.sections[index]
    }

    /// Returns the total size required for the buffer, including all sections and paddings.
    pub fn total_size(&self) -> usize {
        self.sections.last().map_or(0, |s| s.offset + s.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_alignment() {
        let mut layout = BufferLayout::new();
        // Section 0: size 10, offset 0
        layout.add::<u8>(10, 1);
        // Section 1: alignment 64, should start at 64
        layout.add::<u8>(10, 64);

        assert_eq!(layout.get_section(0).offset, 0);
        assert_eq!(layout.get_section(1).offset, 64);
        assert_eq!(layout.total_size(), 74);
    }
}
