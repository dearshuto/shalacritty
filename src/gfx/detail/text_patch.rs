use super::CharacterInfoData;

pub struct BufferPatch {
    pub binary: Vec<u8>,
    pub partial_sizes: [usize; 8],
    pub src_offsets: [usize; 8],
    pub dst_offsets: [usize; 8],
    pub count: u8,
}

pub struct CharacterDataPatch {
    pub index: usize,
    pub transform0: Option<[f32; 4]>,
    pub transform1: Option<[f32; 4]>,
    pub foreground_color: Option<[f32; 4]>,
    pub uv_bl: Option<[f32; 2]>,
    pub uv_tr: Option<[f32; 2]>,
}

impl From<CharacterDataPatch> for BufferPatch {
    fn from(value: CharacterDataPatch) -> Self {
        let mut binary = Vec::default();
        let mut partial_sizes = [0; 8];
        let mut src_offsets = [0; 8];
        let mut dst_offsets = [0; 8];
        let mut current_index = 0;

        if let Some(transform0) = value.transform0 {
            let partial_binary = bytemuck::cast_slice(&transform0);
            partial_sizes[current_index] = partial_binary.len();
            src_offsets[current_index] = binary.len();
            dst_offsets[current_index] = value.index * std::mem::size_of::<CharacterInfoData>();
            binary.extend_from_slice(partial_binary);
            current_index += 1;
        }

        if let Some(transform1) = value.transform1 {
            let partial_binary = bytemuck::cast_slice(&transform1);
            partial_sizes[current_index] = partial_binary.len();
            src_offsets[current_index] = binary.len();
            dst_offsets[current_index] = value.index * std::mem::size_of::<CharacterInfoData>()
                + std::mem::size_of::<[f32; 4]>();
            binary.extend_from_slice(partial_binary);
            current_index += 1;
        }

        if let Some(foreground_color) = value.foreground_color {
            let partial_binary = bytemuck::cast_slice(&foreground_color);
            partial_sizes[current_index] = partial_binary.len();
            src_offsets[current_index] = binary.len();
            dst_offsets[current_index] = value.index * std::mem::size_of::<CharacterInfoData>()
                + std::mem::size_of::<[f32; 4]>()
                + std::mem::size_of::<[f32; 4]>();
            binary.extend_from_slice(partial_binary);
            current_index += 1;
        }

        if let Some(uv_bl) = value.uv_bl {
            let partial_binary = bytemuck::cast_slice(&uv_bl);
            partial_sizes[current_index] = partial_binary.len();
            src_offsets[current_index] = binary.len();
            dst_offsets[current_index] = value.index * std::mem::size_of::<CharacterInfoData>()
                + std::mem::size_of::<[f32; 4]>()
                + std::mem::size_of::<[f32; 4]>()
                + std::mem::size_of::<[f32; 4]>();
            binary.extend_from_slice(partial_binary);
            current_index += 1;
        }

        if let Some(uv_tr) = value.uv_tr {
            let partial_binary = bytemuck::cast_slice(&uv_tr);
            partial_sizes[current_index] = partial_binary.len();
            src_offsets[current_index] = binary.len();
            dst_offsets[current_index] = value.index * std::mem::size_of::<CharacterInfoData>()
                + std::mem::size_of::<[f32; 4]>()
                + std::mem::size_of::<[f32; 4]>()
                + std::mem::size_of::<[f32; 4]>()
                + std::mem::size_of::<[f32; 4]>();
            binary.extend_from_slice(partial_binary);
            current_index += 1;
        }

        Self {
            binary,
            partial_sizes,
            src_offsets,
            dst_offsets,
            count: current_index as u8,
        }
    }
}
