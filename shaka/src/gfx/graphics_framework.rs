use ash::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::gfx::vkutil;

#[derive(Debug, Clone, Copy, Default)]
pub struct CommandBufferHandle {}

pub struct GraphicsFramework {
    instance: ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    queue: vk::Queue,
    #[allow(unused)]
    transfer_queue: vk::Queue,
    debug_utils: Option<vkutil::DebugUtils>,

    surface: vk::SurfaceKHR,
    surface_loader: khr::surface::Instance,
    swapchain_loader: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    #[allow(unused)]
    swapchain_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,

    display_semaphores: Vec<vk::Semaphore>,
    command_completed_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    command_buffers: [vk::CommandBuffer; 2],

    frame_count: u64,
}

impl GraphicsFramework {
    pub fn new<T>(window: T) -> Self
    where
        T: HasWindowHandle + HasDisplayHandle,
    {
        let entry = ash::Entry::linked();
        let instance = {
            let application_info = vkutil::ApplicationInfoFactory::create();
            let extension_names: Vec<_> = ash_window::enumerate_required_extensions(
                window.display_handle().unwrap().as_raw(),
            )
            .unwrap()
            .to_vec()
            .into_iter()
            .chain(
                [
                    ash::ext::debug_utils::NAME.as_ptr(),
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                    khr::get_physical_device_properties2::NAME.as_ptr(),
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                    khr::portability_enumeration::NAME.as_ptr(),
                ]
                .into_iter(),
            )
            .collect();

            let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                ash::vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                ash::vk::InstanceCreateFlags::default()
            };
            let layer_names = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
            unsafe {
                entry.create_instance(
                    &vk::InstanceCreateInfo::default()
                        .application_info(&application_info)
                        .enabled_layer_names(&layer_names)
                        .enabled_extension_names(&extension_names)
                        .flags(create_flags),
                    None,
                )
            }
            .unwrap()
        };
        let debug_utils = vkutil::DebugUtils::new(&entry, &instance);

        // サーフェイス
        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None,
            )
        }
        .unwrap();

        // 物理デバイスの検索
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        let device_capability = vkutil::search_device_capability(&instance);

        // デバイス作成
        let device = unsafe {
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
                ash::khr::swapchain::NAME.as_ptr(),
                ash::khr::storage_buffer_storage_class::NAME.as_ptr(),
                ash::khr::dynamic_rendering::NAME.as_ptr(),
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
                    if device_capability.graphics_queue_index
                        != device_capability.transfer_queue_index
                    {
                        &queue_infos
                    } else {
                        std::slice::from_ref(&queue_infos[device_capability.graphics_queue_index])
                    },
                )
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features)
                .push_next(&mut vulkan_11_features)
                .push_next(&mut vulkan_12_features)
                .push_next(&mut vulkan_13_features);
            ash::vk::DeviceCreateFlags::default();

            instance.create_device(device_capability.physical_device, &device_create_info, None)
        }
        .unwrap();

        let queue =
            unsafe { device.get_device_queue(device_capability.graphics_queue_index as u32, 0) };
        let transfer_queue = if device_capability.transfer_queue_index
            == device_capability.graphics_queue_index
        {
            if device_capability.graphics_queue_count == 1 {
                queue
            } else {
                // Graphics に Transfer が相乗りする場合でも、複数のキューが存在するなら可能な限り処理を分離するためにそれぞれ割り当てる
                unsafe { device.get_device_queue(device_capability.transfer_queue_index as u32, 1) }
            }
        } else {
            unsafe { device.get_device_queue(device_capability.transfer_queue_index as u32, 0) }
        };

        let surface_format = unsafe {
            surface_loader
                .get_physical_device_surface_formats(device_capability.physical_device, surface)
        }
        .unwrap()[0];

        let surface_capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(
                device_capability.physical_device,
                surface,
            )
        }
        .unwrap();

        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: 1280,
                height: 960,
            },
            _ => surface_capabilities.current_extent,
        };
        // スワップチェーン
        // MEMO: 物理デバイスに問い合わせて他の選択肢をとることもできるが、
        // どのプラットフォームでも動作することを期待できる FIFO 方式にしておく
        let present_mode = ash::vk::PresentModeKHR::FIFO;
        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(surface_capabilities.min_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(ash::vk::SurfaceTransformFlagsKHR::IDENTITY) // 回転不要
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);
        let swapchain =
            unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }.unwrap();

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }.unwrap();
        let present_image_views: Vec<_> = swapchain_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                unsafe { device.create_image_view(&create_view_info, None) }.unwrap()
            })
            .collect();

        let (display_semaphores, command_completed_semaphores, command_semaphore) = {
            let create_info = vk::SemaphoreCreateInfo::default();
            let mut semaphore_type_create_info = vk::SemaphoreTypeCreateInfo::default()
                .semaphore_type(vk::SemaphoreType::TIMELINE)
                .initial_value(0);
            let timeline_semaphore_create_info =
                vk::SemaphoreCreateInfo::default().push_next(&mut semaphore_type_create_info);
            let display_semaphores = (0..swapchain_images.len())
                .map(|_| unsafe { device.create_semaphore(&create_info, None) }.unwrap())
                .collect();
            let command_completed_semaphores = (0..swapchain_images.len())
                .map(|_| unsafe { device.create_semaphore(&create_info, None) }.unwrap())
                .collect();
            let command_semaphore = unsafe {
                device
                    .create_semaphore(&timeline_semaphore_create_info, None)
                    .unwrap()
            };
            (
                display_semaphores,
                command_completed_semaphores,
                command_semaphore,
            )
        };

        let in_flight_fences = {
            let info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
            (0..swapchain_images.len())
                .map(|_| unsafe { device.create_fence(&info, None).unwrap() })
                .collect::<Vec<_>>()
        };

        Self {
            instance,
            device,
            physical_device: device_capability.physical_device,
            queue,
            transfer_queue,
            debug_utils: Some(debug_utils),
            surface,
            surface_loader,
            swapchain_loader,
            swapchain,
            swapchain_images,
            present_image_views,
            display_semaphores,
            command_completed_semaphores,
            in_flight_fences,
            command_buffers: [vk::CommandBuffer::null(); 2],
            frame_count: 0,
        }
    }

    pub fn allocate_command_buffer(&mut self) -> CommandBufferHandle {
        CommandBufferHandle {}
    }

    pub fn render(&mut self) {
        self.frame_count += 1;

        let object_index = (self.frame_count % 2) as usize;
        let device = &self.device;
        let display_semaphore = self.display_semaphores[object_index];
        let command_buffer = self.command_buffers[object_index];
        let command_completed_semaphore = self.command_completed_semaphores[object_index];
        let in_flight_fence = self.in_flight_fences[object_index];

        let (next_frame_index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                display_semaphore,
                vk::Fence::null(),
            )
        }
        .unwrap();

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &[ash::vk::ImageMemoryBarrier::default()
                    .old_layout(ash::vk::ImageLayout::UNDEFINED)
                    .new_layout(ash::vk::ImageLayout::ATTACHMENT_OPTIMAL)
                    .image(self.swapchain_images[next_frame_index as usize])
                    .subresource_range(
                        ash::vk::ImageSubresourceRange::default()
                            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    )
                    .dst_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE)],
            )
        };

        let color_attachments = [vk::RenderingAttachmentInfo::default()
            .image_view(self.present_image_views[next_frame_index as usize])
            .image_layout(ash::vk::ImageLayout::ATTACHMENT_OPTIMAL)
            // 背景にかならず画面を覆う矩形を用意しているのでクリアしなくてもよい
            .load_op(ash::vk::AttachmentLoadOp::DONT_CARE)];
        let begin_info = ash::vk::RenderingInfo::default()
            // TODO
            // .render_area(ash::vk::Rect2D::default().extent(self.surface_resolution))
            .layer_count(1)
            .color_attachments(&color_attachments);
        unsafe {
            device.cmd_begin_rendering(command_buffer, &begin_info);

            // TODO: ここでユーザーの描画コマンドを構築する
            device.cmd_execute_commands(command_buffer, &[]);

            device.cmd_pipeline_barrier(
                command_buffer,
                ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ash::vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &[ash::vk::ImageMemoryBarrier::default()
                    .old_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
                    .image(self.swapchain_images[next_frame_index as usize])
                    .subresource_range(
                        ash::vk::ImageSubresourceRange::default()
                            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    )
                    .src_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_access_mask(ash::vk::AccessFlags::MEMORY_READ)],
            );

            device.end_command_buffer(command_buffer).unwrap();

            let command_buffer_infos =
                [vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer)];
            let wait_semaphore_infos = [vk::SemaphoreSubmitInfo::default()
                .semaphore(display_semaphore)
                .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];
            let signal_semaphore_infos = [
                // コマンドバッファー同期用のセマフォの更新
                vk::SemaphoreSubmitInfo::default()
                    // .semaphore(self.command_semaphore)
                    .value(self.frame_count)
                    .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS),
                // ディスプレイの Present 用のセマフォをシグナル
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(command_completed_semaphore)
                    .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS),
            ];
            let submit_infos = [vk::SubmitInfo2::default()
                .command_buffer_infos(&command_buffer_infos)
                .wait_semaphore_infos(&wait_semaphore_infos)
                .signal_semaphore_infos(&signal_semaphore_infos)];
            device
                .queue_submit2(self.queue, &submit_infos, in_flight_fence)
                .unwrap();

            {
                let wait_semaphores = [command_completed_semaphore];
                let swapchain = [self.swapchain];
                let image_indices = [next_frame_index];
                let present_info = vk::PresentInfoKHR::default()
                    .wait_semaphores(&wait_semaphores)
                    .swapchains(&swapchain)
                    .image_indices(&image_indices);

                self.swapchain_loader
                    .queue_present(self.queue, &present_info)
                    .unwrap();
            }
        }
    }
}

impl Drop for GraphicsFramework {
    fn drop(&mut self) {
        // フェンス
        while let Some(fence) = self.in_flight_fences.pop() {
            unsafe { self.device.destroy_fence(fence, None) }
        }

        // コマンド同期 (GPU)
        while let Some(semaphore) = self.command_completed_semaphores.pop() {
            unsafe {
                self.device.destroy_semaphore(semaphore, None);
            }
        }

        // ディスプレイ同期
        while let Some(semaphore) = self.command_completed_semaphores.pop() {
            unsafe {
                self.device.destroy_semaphore(semaphore, None);
            }
        }

        while let Some(image_view) = self.present_image_views.pop() {
            unsafe { self.device.destroy_image_view(image_view, None) };
        }

        // スワップチェーン
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };

        // サーフェイス
        unsafe { self.surface_loader.destroy_surface(self.surface, None) };

        // デバッグユーティリティー
        // Instance の覇気よりも先に破棄
        self.debug_utils.take().unwrap();

        unsafe { self.device.destroy_device(None) };
        unsafe { self.instance.destroy_instance(None) };
    }
}
