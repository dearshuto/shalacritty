use ash::*;

pub struct OffscreenTarget {
    image: vk::Image,
    image_view: vk::ImageView,
    allocation: Option<gpu_allocator::vulkan::Allocation>,
    extent: vk::Extent2D,
}

impl OffscreenTarget {
    /// レンダーターゲットの解像度を取得
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub(crate) fn image(&self) -> vk::Image {
        self.image
    }

    /// 描画（Dynamic Rendering）に必要な ImageView を取得
    pub(crate) fn image_view(&self) -> vk::ImageView {
        self.image_view
    }
}

pub struct OffscreenTargetFactory;

impl OffscreenTargetFactory {
    pub fn create(
        device: &ash::Device,
        width: u32,
        height: u32,
        allocator: &mut gpu_allocator::vulkan::Allocator,
    ) -> OffscreenTarget {
        let extent = vk::Extent2D { width, height };

        // 1. VkImage の生成
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { device.create_image(&image_info, None).unwrap() };

        // 2. 中央アロケータからのメモリ確保 & バインド
        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
        let allocation = allocator
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Offscreen Target Image",
                requirements: memory_requirements,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .unwrap();

        unsafe {
            device
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .unwrap();
        }

        // 3. VkImageView の生成
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe { device.create_image_view(&view_info, None).unwrap() };

        OffscreenTarget {
            image,
            image_view,
            allocation: Some(allocation),
            extent,
        }
    }
}
