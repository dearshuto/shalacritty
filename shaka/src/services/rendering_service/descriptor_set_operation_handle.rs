use ash::*;

use crate::services::rendering_service::transfer_queue::TransferQueue;

pub enum ImageFormat {
    R32g32b32Float,
}

pub struct ImageData {
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub raw: u32,
    pub format: ImageFormat,
}

pub struct DescriptorSetOperationHandle {
    device: ash::Device,
    // データ転送用のキュー
    queue: vk::Queue,
    // データ転送コマンドを積むバッファー
    command_buffer: vk::CommandBuffer,
    // データ転送完了待ちフェンス
    fence: vk::Fence,
    sampler: vk::Sampler,

    // 定数バッファー
    uniform_buffer: vk::Buffer,
    offset: vk::DeviceSize,
    size: vk::DeviceSize,

    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    sender: tokio::sync::oneshot::Sender<Vec<vk::DescriptorSet>>,

    transfer_queue: TransferQueue<f32>,
}

impl DescriptorSetOperationHandle {
    pub fn new(device: ash::Device, vk::Queue) -> Self {

    }

    pub fn create_descriptor_sets(&mut self, count: usize) {
        let set_layouts = [self.descriptor_set_layout; 8];
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&set_layouts[0..count]);
        self.descriptor_sets =
            unsafe { self.device.allocate_descriptor_sets(&allocate_info) }.unwrap();
    }

    pub fn update_descriptor_set(&mut self, index: usize, data: ImageData) {
        let image = unsafe {
            let create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(Self::to_vk_format(data.format))
                .extent(
                    vk::Extent3D::default()
                        .width(data.width)
                        .height(data.height)
                        .depth(1),
                )
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED);
            self.device.create_image(&create_info, None).unwrap()
        };

        let image_view = unsafe {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D);
            self.device.create_image_view(&create_info, None).unwrap()
        };

        let descriptor_set = self.descriptor_sets[index];
        let sampler_infos = [vk::DescriptorImageInfo::default().sampler(self.sampler)];
        let image_infos = [vk::DescriptorImageInfo::default()
            .image_view(image_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

        let uniform_buffer_infos = [vk::DescriptorBufferInfo::default()
            .buffer(self.uniform_buffer)
            .offset(self.offset)
            .range(self.size)];

        let descriptor_writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&sampler_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(3)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&uniform_buffer_infos),
        ];

        unsafe {
            self.device.update_descriptor_sets(&descriptor_writes, &[]);
        }

        let mut transfer_queue = TransferQueue::new();
        let mut buffer = vec![0f32; 1024];
        transfer_queue.push(&mut [], &data.data);
        while let Some(size) = transfer_queue.pop(&mut buffer) {
            let regions = [vk::BufferImageCopy2::default()
                .image_extent(
                    vk::Extent3D::default()
                        .width(data.width)
                        .height(data.height)
                        .depth(1),
                )
                .image_offset(vk::Offset3D::default())
                .buffer_offset(0)
                .buffer_row_length(data.raw)
                .image_subresource(
                    vk::ImageSubresourceLayers::default()
                        .base_array_layer(0)
                        .layer_count(1)
                        .mip_level(0)
                        .aspect_mask(vk::ImageAspectFlags::COLOR),
                )];
            let buffer_image_copy = vk::CopyBufferToImageInfo2::default()
                .dst_image(image)
                .dst_image_layout(vk::ImageLayout::UNDEFINED)
                .src_buffer(vk::Buffer::null())
                .regions(&regions);
            unsafe {
                self.device
                    .cmd_copy_buffer_to_image2(vk::CommandBuffer::null(), &buffer_image_copy)
            };

            let command_buffer_infos =
                [vk::CommandBufferSubmitInfo::default().command_buffer(self.command_buffer)];
            let submit_infos =
                [vk::SubmitInfo2::default().command_buffer_infos(&command_buffer_infos)];
            unsafe {
                self.device
                    .queue_submit2(self.queue, &submit_infos, self.fence)
            }
            .unwrap();
        }
    }

    pub fn finish(self) {
        self.sender.send(self.descriptor_sets).ok();
    }

    fn to_vk_format(image_format: ImageFormat) -> vk::Format {
        match image_format {
            ImageFormat::R32g32b32Float => vk::Format::R32G32B32_SFLOAT,
        }
    }
}
