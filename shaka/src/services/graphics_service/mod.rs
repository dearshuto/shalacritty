use std::{borrow::Cow, collections::HashMap};

use ash::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct GraphicsService {
    instance: ash::Instance,
    device: ash::Device,
    queue: vk::Queue,
    debug_utils_loader: ext::debug_utils::Instance,
    debug_utils_messanger: vk::DebugUtilsMessengerEXT,

    dynamic_rendering_device: khr::dynamic_rendering::Device,

    // Graphics Framework
    surface: vk::SurfaceKHR,
    surface_loader: khr::surface::Instance,
    swapchain_loader: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,

    // フレーム同期
    display_semaphores: Vec<vk::Semaphore>,
    command_completed_semaphores: Vec<vk::Semaphore>,
    command_fences: Vec<vk::Fence>,

    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
}

impl GraphicsService {
    pub fn new<T>(window: T) -> Self
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
                ash::khr::dynamic_rendering::NAME.as_ptr(),
                ash::khr::synchronization2::NAME.as_ptr(),
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                ash::khr::portability_subset::NAME.as_ptr(),
            ];
            let mut vulkan_features =
                vk::PhysicalDeviceVulkan11Features::default().shader_draw_parameters(true);
            let mut dynamic_rendering_features =
                ash::vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);
            let mut sync_features =
                ash::vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true);
            let device_create_info = ash::vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features)
                .push_next(&mut vulkan_features)
                .push_next(&mut dynamic_rendering_features)
                .push_next(&mut sync_features);
            ash::vk::DeviceCreateFlags::default();

            instance.create_device(physical_device, &device_create_info, None)
        }
        .unwrap();

        let dynamic_rendering_device = khr::dynamic_rendering::Device::new(&instance, &device);

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
        let command_pool = {
            let create_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            unsafe { device.create_command_pool(&create_info, None) }.unwrap()
        };

        let command_buffers = {
            let allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(2);
            unsafe { device.allocate_command_buffers(&allocate_info) }.unwrap()
        };

        let command_fences = {
            let create_info = vk::FenceCreateInfo::default();
            let fence0 = unsafe { device.create_fence(&create_info, None) }.unwrap();
            let fence1 = unsafe { device.create_fence(&create_info, None) }.unwrap();
            vec![fence0, fence1]
        };

        let (display_semaphores, command_completed_semaphores) = {
            let create_info = vk::SemaphoreCreateInfo::default();
            let display_semaphore0 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            let display_semaphore1 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();

            let command_completed_semaphore0 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            let command_completed_semaphore1 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            (
                vec![display_semaphore0, display_semaphore1],
                vec![command_completed_semaphore0, command_completed_semaphore1],
            )
        };

        Self {
            instance,
            device,
            queue,
            debug_utils_loader,
            debug_utils_messanger: debug_utils,
            dynamic_rendering_device,
            surface,
            surface_loader,
            swapchain_loader,
            swapchain,
            swapchain_images,
            present_image_views,
            display_semaphores,
            command_completed_semaphores,
            command_fences,
            command_pool,
            command_buffers,
        }
    }

    pub fn create_proxy(&mut self) -> GraphicsServiceProxy {
        GraphicsServiceProxy {}
    }

    async fn serve(self) {}

    fn allocate_buffer(&mut self, info: vk::BufferCreateInfo) {
        let buffer = unsafe { self.device.create_buffer(&info, None) };
    }

    extern "system" fn vulkan_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32 {
        let callback_data = unsafe { *p_callback_data };
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            unsafe { std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() }
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            unsafe { std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy() }
        };

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
        );

        vk::FALSE
    }
}

pub struct GraphicsServiceFacade<T, O> {
    sender: tokio::sync::mpsc::Sender<T>,
    _phantom: std::marker::PhantomData<O>,
}

impl<T, O> GraphicsServiceFacade<T, O> {
    pub fn new() -> Self {
        todo!()
    }

    pub async fn request(self, request: T) -> O {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request_arapter = ShaderModuleRequestAdapter { sender };
        self.sender.send(request).await;
        receiver.await.unwrap()
    }
}

pub struct GraphicsServiceProxy {
    shader_module_request_sender:
        tokio::sync::mpsc::Sender<ShaderModuleRequestAdapter<vk::ShaderModule>>,
}

impl GraphicsServiceProxy {
    pub async fn request_image(&self, request: ImageRequest) -> ImageHandle {
        todo!()
    }

    pub async fn request_shader_module(&self, request: ShaderModuleRequest) -> vk::ShaderModule {
        let facade = GraphicsServiceFacade::new();
        facade.request(request).await
    }

    pub async fn request_pipeline_layout(&self) -> vk::PipelineLayout {
        vk::PipelineLayout::null()
    }

    pub async fn request_graphics_pipeline(
        &self,
        request: GraphicsPipelineRequest,
    ) -> [vk::Pipeline; 8] {
        [vk::Pipeline::null(); 8]
    }
}

pub struct ShaderModuleRequest {}

pub struct ShaderModuleRequestAdapter<T> {
    sender: tokio::sync::oneshot::Sender<T>,
}

pub struct GraphicsPipelineRequest {}

pub struct ImageRequest {}

pub struct ImageHandle {
    image: ash::vk::Image,
}
