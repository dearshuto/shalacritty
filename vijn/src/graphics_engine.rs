use ash::*;

use crate::vkutil::{self, DeviceCapability};

pub struct TransferWorkerContext {
    device: ash::Device,
    // CommandPool の所有権のみを持つ (Queue は持たない！)
    command_pool: vk::CommandPool,
}

pub struct OffscreenTarget {
    image: vk::Image,
    image_view: vk::ImageView,
    allocation: Option<gpu_allocator::vulkan::Allocation>,
    extent: vk::Extent2D,
}

impl OffscreenTarget {
    /// 描画（Dynamic Rendering）に必要な ImageView を取得
    pub fn image_view(&self) -> vk::ImageView {
        self.image_view
    }

    /// レンダーターゲットの解像度を取得
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn image(&self) -> vk::Image {
        self.image
    }
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

        let image = unsafe { self.device.create_image(&image_info, None).unwrap() };

        // 2. 中央アロケータからのメモリ確保 & バインド
        let memory_requirements = unsafe { self.device.get_image_memory_requirements(image) };
        let allocation = self
            .allocator
            .lock()
            .unwrap()
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "Offscreen Target Image",
                requirements: memory_requirements,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .unwrap();

        unsafe {
            self.device
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

        let image_view = unsafe { self.device.create_image_view(&view_info, None).unwrap() };

        OffscreenTarget {
            image,
            image_view,
            allocation: Some(allocation),
            extent,
        }
    }

    /// OffscreenTarget の描画結果を CPU 側へ読み戻す (PNG 出力・デバッグ用)
    pub fn readback_offscreen_target(
        &self,
        target: &OffscreenTarget,
        cmd: vk::CommandBuffer,
    ) -> Vec<u8> {
        let width = target.extent.width;
        let height = target.extent.height;
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
                .image(target.image)
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
                target.image,
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

    /// 明示的な OffscreenTarget の破棄処理
    pub fn destroy_offscreen_target(&self, mut target: OffscreenTarget) {
        unsafe {
            self.device.destroy_image_view(target.image_view, None);
            self.device.destroy_image(target.image, None);
        }
        if let Some(allocation) = target.allocation.take() {
            self.allocator.lock().unwrap().free(allocation).unwrap();
        }
    }

    pub fn create_swapchain_target(
        &self,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        window_width: u32,
        window_height: u32,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> SwapchainTarget {
        // 1. Surface の機能・フォーマット・Present Mode の問い合わせ
        let capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap()
        };
        let formats = unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap()
        };

        // フォーマットと Extent の決定
        let format = formats.first().cloned().unwrap_or_default();
        let extent = vk::Extent2D {
            width: window_width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: window_height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        };

        // 2. VkSwapchainCreateInfoKHR の構築
        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(capabilities.min_image_count + 1)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO) // V-Sync 有効
            .clipped(true);

        if let Some(old) = old_swapchain {
            create_info = create_info.old_swapchain(old);
        }

        let swapchain = unsafe {
            self.swapchain_loader
                .create_swapchain(&create_info, None)
                .unwrap()
        };

        // 3. Swapchain Image & ImageView の取得と生成
        let images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(swapchain)
                .unwrap()
        };
        let image_views = images
            .iter()
            .map(|&image| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                unsafe { self.device.create_image_view(&view_info, None).unwrap() }
            })
            .collect();

        SwapchainTarget {
            swapchain,
            images,
            image_views,
            format: format.format,
            extent,
        }
    }

    /// フレームの開始: 次の Swapchain Image インデックスを取得
    pub fn acquire_next_image(
        &self,
        target: &SwapchainTarget,
        semaphore: vk::Semaphore,
    ) -> Result<(u32, bool), vk::Result> {
        unsafe {
            self.swapchain_loader.acquire_next_image(
                target.swapchain,
                u64::MAX,
                semaphore,
                vk::Fence::null(),
            )
        }
    }

    /// フレームの終了: Queue へ Present 要求
    pub fn present(
        &self,
        target: &SwapchainTarget,
        image_index: u32,
        wait_semaphore: vk::Semaphore,
    ) -> Result<bool, vk::Result> {
        let swapchains = [target.swapchain];
        let image_indices = [image_index];
        let wait_semaphores = [wait_semaphore];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)
        }
    }

    /// ウインドウリサイズ時などの Swapchain 再構築処理
    pub fn recreate_swapchain_target(
        &self,
        target: &mut SwapchainTarget,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
        new_width: u32,
        new_height: u32,
    ) {
        unsafe { self.device.device_wait_idle().unwrap() };

        let old_swapchain = target.swapchain;
        let old_image_views = std::mem::take(&mut target.image_views);

        // 新しい SwapchainTarget の作成
        *target = self.create_swapchain_target(
            surface,
            physical_device,
            new_width,
            new_height,
            Some(old_swapchain),
        );

        // 古いリソースの破棄
        unsafe {
            for view in old_image_views {
                self.device.destroy_image_view(view, None);
            }
            self.swapchain_loader.destroy_swapchain(old_swapchain, None);
        }
    }

    /// SwapchainTarget の破棄
    pub fn destroy_swapchain_target(&self, target: SwapchainTarget) {
        unsafe {
            for view in target.image_views {
                self.device.destroy_image_view(view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(target.swapchain, None);
        }
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
