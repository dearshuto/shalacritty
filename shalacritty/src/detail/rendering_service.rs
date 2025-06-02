use std::borrow::Cow;

use ash::*;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tracing::instrument::WithSubscriber;
use winit::window::WindowId;

use crate::{app::WindowSizeChangedEventArgs, Config};

use super::ImageLoadedEventArgs;

pub struct RenderingService<'a> {
    device: ash::Device,
    queue: vk::Queue,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    descriptor_pool: vk::DescriptorPool,
    sampler: vk::Sampler,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    present_image_views: Vec<vk::ImageView>,
    command_fence: vk::Fence,
    display_semaphore: vk::Semaphore,
    acquire_image_semaphore: vk::Semaphore,

    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    image_receiver: tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
    surface: Option<wgpu::Surface<'a>>,
}

impl<'a> RenderingService<'a> {
    pub fn new<T>(
        window: T,
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
        image_receiver: tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>,
    ) -> Self
    where
        T: HasWindowHandle + HasDisplayHandle,
    {
        let entry = ash::Entry::linked();
        let instance = {
            let application_info = vk::ApplicationInfo::default()
                .application_name(c"shalacritty")
                .engine_name(c"shalacritty")
                .application_version(0)
                .engine_version(0)
                .api_version(vk::API_VERSION_1_3);
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

        let debug_utils_loader = ext::debug_utils::Instance::new(&entry, &instance);
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(Self::vulkan_debug_callback));
        let debug_utils =
            unsafe { debug_utils_loader.create_debug_utils_messenger(&debug_info, None) }.unwrap();

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
        let (physical_device, queue_family_index) =
            unsafe { instance.enumerate_physical_devices() }
                .unwrap()
                .iter()
                .find_map(|physical_device| {
                    unsafe {
                        instance.get_physical_device_queue_family_properties(*physical_device)
                    }
                    .iter()
                    .enumerate()
                    .find_map(|(index, info)| {
                        if !info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                            return None;
                        }

                        if !unsafe {
                            surface_loader.get_physical_device_surface_support(
                                *physical_device,
                                index as u32,
                                surface,
                            )
                        }
                        .unwrap()
                        {
                            return None;
                        }

                        Some((*physical_device, index))
                    })
                })
                .unwrap();

        // デバイス作成
        let device = unsafe {
            let features = ash::vk::PhysicalDeviceFeatures::default().shader_clip_distance(true);
            let priorities = [1.0];
            let queue_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .queue_priorities(&priorities);
            let device_extension_names_raw = [
                ash::khr::swapchain::NAME.as_ptr(),
                ash::khr::storage_buffer_storage_class::NAME.as_ptr(),
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                ash::khr::portability_subset::NAME.as_ptr(),
            ];
            let device_create_info = ash::vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);
            ash::vk::DeviceCreateFlags::default();

            instance.create_device(physical_device, &device_create_info, None)
        }
        .unwrap();

        let queue = unsafe { device.get_device_queue(queue_family_index as u32, 0) };
        let surface_format =
            unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }
                .unwrap()[0];

        let surface_capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
        }
        .unwrap();

        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: 640,
                height: 480,
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

        let command_pool = {
            let command_pool_create_info = ash::vk::CommandPoolCreateInfo::default()
                .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index as u32);
            unsafe { device.create_command_pool(&command_pool_create_info, None) }.unwrap()
        };

        // コマンドバッファー
        let command_buffers = {
            let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(2)
                .command_pool(command_pool)
                .level(ash::vk::CommandBufferLevel::PRIMARY);
            unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }.unwrap()
        };

        // デスクリプタープール
        // 決め打ちでバッファー領域を適当に確保
        let descriptor_sizes = [ash::vk::DescriptorPoolSize::default()
            .ty(ash::vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(16)];
        let descriptor_pool_info = ash::vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_sizes)
            .flags(ash::vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1);
        let descriptor_pool =
            unsafe { device.create_descriptor_pool(&descriptor_pool_info, None) }.unwrap();

        // サンプラー
        let sampler = unsafe {
            let create_info = ash::vk::SamplerCreateInfo::default()
                .mag_filter(ash::vk::Filter::LINEAR)
                .min_filter(ash::vk::Filter::LINEAR)
                .mipmap_mode(ash::vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(ash::vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(ash::vk::SamplerAddressMode::CLAMP_TO_EDGE);
            device.create_sampler(&create_info, None)
        }
        .unwrap();

        // フェンス
        let command_fence = {
            let fence_create_info =
                ash::vk::FenceCreateInfo::default().flags(ash::vk::FenceCreateFlags::SIGNALED);
            unsafe { device.create_fence(&fence_create_info, None) }.unwrap()
        };

        // 描画フレームワーク用のセマフォ
        let (display_semaphore, acquire_image_semaphore) = {
            let create_info = vk::SemaphoreCreateInfo::default();
            let display_semaphore = unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            let acquire_image_semaphore =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            (display_semaphore, acquire_image_semaphore)
        };

        let render_pass = {
            let create_info = vk::RenderPassCreateInfo::default();
            unsafe { device.create_render_pass(&create_info, None) }.unwrap()
        };

        let pipeline = {
            let create_info = vk::GraphicsPipelineCreateInfo::default();
            unsafe {
                device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
            }
            .unwrap()
        }[0];

        Self {
            device,
            queue,
            render_pass,
            pipeline,
            descriptor_pool,
            sampler,
            command_pool,
            command_buffer: command_buffers[0],
            // 描画フレームワーク
            present_image_views,
            command_fence,
            display_semaphore,
            acquire_image_semaphore,

            config_receiver,
            window_size_receiver,
            image_receiver,
            redraw_requested_window_id_receiver,
            surface: None, /*Some(surface)*/
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(_config) = self.config_receiver.recv() => {},
            Some(args) = self.window_size_receiver.recv() => self.try_resize(args).await,
            Some(_args) = self.image_receiver.recv() => {},
            Some(window_id) = self.redraw_requested_window_id_receiver.recv() => self.redraw(window_id).await,
                                                    else => break,
                                                );
        }
    }

    async fn try_resize(&mut self, args: WindowSizeChangedEventArgs) {}

    async fn redraw(&mut self, id: WindowId) {}

    unsafe extern "system" fn vulkan_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32 {
        let callback_data = *p_callback_data;
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

        vk::FALSE
    }
}
