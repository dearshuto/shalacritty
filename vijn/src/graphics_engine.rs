use ash::*;

use crate::{
    OffscreenTarget,
    offscreen_target::OffscreenTargetFactory,
    vkutil::{self, DeviceCapability},
};

pub struct TransferWorkerContext {
    device: ash::Device,
    // CommandPool の所有権のみを持つ (Queue は持たない！)
    command_pool: vk::CommandPool,
}

pub struct SwapchainTarget {
    pub(crate) swapchain: vk::SwapchainKHR,
    pub(crate) images: Vec<vk::Image>,
    pub(crate) image_views: Vec<vk::ImageView>,
    pub(crate) format: vk::Format,
    pub(crate) extent: vk::Extent2D,
}

impl SwapchainTarget {
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn format(&self) -> vk::Format {
        self.format
    }

    /// フレームごとの Image / ImageView を取得
    pub fn image_view(&self, index: usize) -> vk::ImageView {
        self.image_views[index]
    }
}

pub struct GraphicsEngine {
    device: ash::Device,
    transfer_queue_family_index: u32,
}

impl GraphicsEngine {
    pub fn new(instance: &ash::Instance, device_capability: &DeviceCapability) -> Self {
        let features = ash::vk::PhysicalDeviceFeatures::default().shader_clip_distance(true);
        let priorities = [1.0];
        let queue_infos = [
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(device_capability.graphics_queue_index as u32)
                .queue_priorities(&priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(device_capability.transfer_queue_index as u32)
                .queue_priorities(&priorities),
        ];
        let device_extension_names_raw = [
            ash::khr::storage_buffer_storage_class::NAME.as_ptr(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            ash::khr::portability_subset::NAME.as_ptr(),
        ];
        let mut vulkan_11_features =
            vk::PhysicalDeviceVulkan11Features::default().shader_draw_parameters(true);
        let mut vulkan_12_features =
            vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
        let mut vulkan_13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true);

        let device_create_info = ash::vk::DeviceCreateInfo::default()
            // キューファミリーインデックスはユニークでないといけないので、
            // Transfer キューが Graphics キューに相乗りしてる場合はグラフィックスだけが設定されるようにする
            .queue_create_infos(
                if device_capability.graphics_queue_index != device_capability.transfer_queue_index
                {
                    &queue_infos
                } else {
                    std::slice::from_ref(
                        &queue_infos[device_capability.graphics_queue_index as usize],
                    )
                },
            )
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features)
            .push_next(&mut vulkan_11_features)
            .push_next(&mut vulkan_12_features)
            .push_next(&mut vulkan_13_features);
        ash::vk::DeviceCreateFlags::default();

        let device = unsafe {
            instance
                .create_device(device_capability.physical_device, &device_create_info, None)
                .unwrap()
        };

        let device_capability = vkutil::search_device_capability(&instance);

        Self {
            device,
            transfer_queue_family_index: device_capability.transfer_queue_index,
        }
    }

    pub fn render(&self) {}

    /// OffscreenTarget の生成ファクトリメソッド
    pub fn create_offscreen_target(&self, width: u32, height: u32) -> OffscreenTarget {
        todo!()
        // OffscreenTargetFactory::create(&self.device, width, height, allocator)
    }

    /// 明示的な OffscreenTarget の破棄処理
    pub fn destroy_offscreen_target(&self, mut target: OffscreenTarget) {
        unsafe {
            self.device.destroy_image_view(target.image_view(), None);
            self.device.destroy_image(target.image(), None);
        }
        if let Some(allocation) = target.allocation.take() {
            self.allocator.lock().unwrap().free(allocation).unwrap();
        }
    }

    /// OffscreenTarget の描画結果を CPU 側へ読み戻す (PNG 出力・デバッグ用)
    pub fn readback_offscreen_target(
        &self,
        target: &OffscreenTarget,
        cmd: vk::CommandBuffer,
    ) -> Vec<u8> {
        let width = target.extent().width;
        let height = target.extent().height;
        let buffer_size = (width * height * 4) as u64; // RGBA 8-bit

        // 1. CPU から読める Staging Buffer を割り当て
        let buffer_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST);

        let staging_buffer = unsafe { self.device.create_buffer(&buffer_info, None).unwrap() };
        let requirements = unsafe { self.device.get_buffer_memory_requirements(staging_buffer) };

        let staging_allocation = self
            .allocator
            .lock()
            .unwrap()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Offscreen Readback Staging Buffer",
                requirements,
                location: gpu_allocator::MemoryLocation::CpuToGpu, // CPUから読めるメモリ
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .unwrap();

        unsafe {
            self.device
                .bind_buffer_memory(
                    staging_buffer,
                    staging_allocation.memory(),
                    staging_allocation.offset(),
                )
                .unwrap();
        }

        // 2. コマンド記録 (Image (COLOR_ATTACHMENT) -> Image (TRANSFER_SRC) -> Staging Buffer)
        unsafe {
            // パイプラインバリアで Layout 変更
            let barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .dst_access_mask(vk::AccessFlags2::TRANSFER_READ)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .image(target.image())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dependency_info =
                vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
            self.device.cmd_pipeline_barrier2(cmd, &dependency_info);

            // Copy Image to Buffer
            let copy_region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });

            self.device.cmd_copy_image_to_buffer(
                target.image(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                staging_buffer,
                &[copy_region],
            );
        }

        // --- (※実運用ではここで Queue Submit & Fence 待ちを行なった後、以下の map/copy を実行) ---

        // 3. メモリの読み出し
        let mut pixel_data = vec![0u8; buffer_size as usize];
        if let Some(mapped_ptr) = staging_allocation.mapped_ptr() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    mapped_ptr.as_ptr() as *const u8,
                    pixel_data.as_mut_ptr(),
                    buffer_size as usize,
                );
            }
        }

        // 4. Staging リソースのクリーンアップ
        unsafe {
            self.device.destroy_buffer(staging_buffer, None);
        }
        self.allocator
            .lock()
            .unwrap()
            .free(staging_allocation)
            .unwrap();

        pixel_data
    }

    pub fn create_transfer_worker(&self) -> TransferWorkerContext {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(self.transfer_queue_family_index)
            // ワーカースレッドで個別にリセットできるようにフラグを設定
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe { self.device.create_command_pool(&pool_info, None).unwrap() };

        // ワーカースレッド専用の文脈（Context）として所有権ごと手渡す
        TransferWorkerContext {
            device: self.device.clone(),
            command_pool,
        }
    }
}

impl Drop for GraphicsEngine {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}
