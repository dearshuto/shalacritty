use std::collections::HashSet;

use ash::*;

use crossfont::RasterizedGlyph;

#[derive(Debug)]
pub struct GlyphRange {}

impl GlyphRange {
    pub fn lower_left(&self) -> [f32; 2] {
        [0.0, 0.0]
    }

    pub fn upper_right(&self) -> [f32; 2] {
        [0.0, 0.0]
    }
}

pub struct GlyphTexture {
    device: ash::Device,
    image: vk::Image,
    image_view: vk::ImageView,
    memory: vk::DeviceMemory,

    // 過去にラスタライズした文字
    glyph_cache: HashSet<char>,
}

impl GlyphTexture {
    pub fn new(device: ash::Device) -> Self {
        let image = {
            let create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8_UINT)
                .extent(vk::Extent3D::default().width(640).height(640).depth(1))
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .queue_family_indices(&[0])
                .initial_layout(vk::ImageLayout::UNDEFINED);
            unsafe { device.create_image(&create_info, None) }
        }
        .unwrap();

        let memory = {
            let requirements = unsafe { device.get_image_memory_requirements(image) };
            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(0); // TODO
            unsafe { device.allocate_memory(&allocate_info, None) }.unwrap()
        };

        unsafe { device.bind_image_memory(image, memory, 0) }.unwrap();

        let image_view = {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8_UINT)
                .components(
                    vk::ComponentMapping::default()
                        .r(vk::ComponentSwizzle::R)
                        .g(vk::ComponentSwizzle::G)
                        .b(vk::ComponentSwizzle::B)
                        .a(vk::ComponentSwizzle::A),
                )
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                );
            unsafe { device.create_image_view(&create_info, None) }
        }
        .unwrap();

        Self {
            device,
            image,
            image_view,
            glyph_cache: HashSet::default(),
            memory,
        }
    }

    pub fn range(&self, _code: char) -> GlyphRange {
        GlyphRange {}
    }

    pub fn write(&self, glyphs: &[RasterizedGlyph]) {
        for glyph in glyphs {
            if self.glyph_cache.contains(&glyph.character) {
                continue;
            }
        }
    }
}

impl Drop for GlyphTexture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.image_view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);

            // デバイスは GlyphTexture の外側で管理されてるので破棄しないこと
            // MEMO: 所有権を明言して破棄できない仕組みを作りたい
        }
    }
}
