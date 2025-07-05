use std::{borrow::Cow, io::Cursor};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use ash::{
    ext::color_write_enable, qcom::render_pass_transform, vk::VertexInputBindingDescription, *,
};
use tracing::instrument::WithSubscriber;

pub struct RenderingServiceVk {
    instance: ash::Instance,
    device: ash::Device,
    surface: vk::SurfaceKHR,
    queue: vk::Queue,

    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    // Graphics Framework
    surface_loader: khr::surface::Instance,
    swapchain_loader: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    command_fence: vk::Fence,
    display_semaphore: vk::Semaphore,
    command_completed_semaphore: vk::Semaphore,

    // Resources
    layout: vk::PipelineLayout,
    buffer: vk::Buffer,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl RenderingServiceVk {
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
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                ash::khr::portability_subset::NAME.as_ptr(),
            ];
            let mut vulkan_features =
                vk::PhysicalDeviceVulkan11Features::default().shader_draw_parameters(true);
            let device_create_info = ash::vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features)
                .push_next(&mut vulkan_features);
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
        let render_pass = {
            let attachment_descriptions = [vk::AttachmentDescription::default()
                .format(surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                // .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
            let color_attachments = [vk::AttachmentReference::default()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
            let subpass_descriptions = [vk::SubpassDescription::default()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&color_attachments)];
            let subpass_dependencies = [vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(
                    vk::AccessFlags::COLOR_ATTACHMENT_READ
                        | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                )
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)];
            let create_info = vk::RenderPassCreateInfo::default()
                .attachments(&attachment_descriptions)
                .subpasses(&subpass_descriptions)
                .dependencies(&subpass_dependencies);
            unsafe { device.create_render_pass(&create_info, None) }.unwrap()
        };

        let framebuffers = {
            present_image_views.iter().map(|image_view| {
                let attachments = [*image_view];
                let create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(640)
                    .height(480)
                    .layers(1);
                unsafe { device.create_framebuffer(&create_info, None) }.unwrap()
            })
        }
        .collect();

        let shader_module = {
            let mut cursor = Cursor::new(include_bytes!("terminal.spv"));
            let code = ash::util::read_spv(&mut cursor).unwrap();
            let create_info = vk::ShaderModuleCreateInfo::default().code(&code);
            unsafe { device.create_shader_module(&create_info, None) }.unwrap()
        };

        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::default();
            unsafe { device.create_pipeline_layout(&create_info, None) }.unwrap()
        };

        let pipeline = {
            let shader_stage_create_info = [
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(shader_module)
                    .name(c"main_vs"),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(shader_module)
                    .name(c"main_fs"),
            ];

            let vertex_binding_descriptions = [vk::VertexInputBindingDescription::default()
                .binding(0)
                .stride(std::mem::size_of::<f32>() as u32 * 2)
                .input_rate(vk::VertexInputRate::VERTEX)];
            let vertex_attribute_descriptions = [vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0)];
            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&vertex_binding_descriptions)
                .vertex_attribute_descriptions(&vertex_attribute_descriptions);
            let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let viewpors = [vk::Viewport::default().width(640.0).height(480.0)];
            let scissors =
                [vk::Rect2D::default().extent(vk::Extent2D::default().width(640).height(480))];
            let viewport_state = vk::PipelineViewportStateCreateInfo::default()
                .viewports(&viewpors)
                .scissors(&scissors);
            let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0);
            let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default();
            let blend_attachment_states = [vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)];
            let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
                .logic_op(vk::LogicOp::CLEAR)
                .attachments(&blend_attachment_states);
            let dynamic_state = vk::PipelineDynamicStateCreateInfo::default();
            let create_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&shader_stage_create_info)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_state)
                .depth_stencil_state(&depth_stencil_state)
                .color_blend_state(&color_blend_state)
                .dynamic_state(&dynamic_state)
                .render_pass(render_pass)
                .layout(layout);

            unsafe {
                device.create_graphics_pipelines(vk::PipelineCache::null(), &[create_info], None)
            }
            .unwrap()[0]
        };

        const BUFFER_SIZE: vk::DeviceSize = 16 * 1024;
        let buffer = {
            let queue_family_indices = [queue_family_index as u32];
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(&queue_family_indices)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .size(BUFFER_SIZE)
                .usage(
                    vk::BufferUsageFlags::VERTEX_BUFFER
                        | vk::BufferUsageFlags::INDEX_BUFFER
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                );
            unsafe { device.create_buffer(&create_info, None) }.unwrap()
        };

        let device_memory = {
            let memory_index = {
                let memory_requirement = unsafe { device.get_buffer_memory_requirements(buffer) };
                unsafe { instance.get_physical_device_memory_properties(physical_device) }
                    .memory_types_as_slice()
                    .iter()
                    .enumerate()
                    .find(|(index, memory_type)| {
                        let flags = vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT;
                        (1 << index) & memory_requirement.memory_type_bits != 0
                            && memory_type.property_flags & flags == flags
                    })
                    .map(|(index, _)| index as u32)
                    .unwrap()
            };
            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(BUFFER_SIZE)
                .memory_type_index(memory_index);
            unsafe { device.allocate_memory(&allocate_info, None) }.unwrap()
        };

        unsafe { device.bind_buffer_memory(buffer, device_memory, 0) }.unwrap();

        let (vertex_buffer, index_buffer) = {
            let vertrex_ptr = unsafe {
                device.map_memory(
                    device_memory,
                    0, /*offset*/
                    1024,
                    vk::MemoryMapFlags::empty(),
                )
            }
            .unwrap() as *mut f32;
            let index_ptr = unsafe { vertrex_ptr.add(16) } as *mut u16;

            (
                unsafe { std::slice::from_raw_parts_mut(vertrex_ptr, 16) },
                unsafe { std::slice::from_raw_parts_mut(index_ptr, 16) },
            )
        };

        const VERTEX_DATA: [f32; 8] = [-0.5f32, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5];
        const INDEX_DATA: [u16; 6] = [0, 1, 2, 0, 2, 3];
        vertex_buffer[0..VERTEX_DATA.len()].copy_from_slice(&VERTEX_DATA);
        index_buffer[0..INDEX_DATA.len()].copy_from_slice(&INDEX_DATA);

        unsafe {
            device.flush_mapped_memory_ranges(&[vk::MappedMemoryRange::default()
                .memory(device_memory)
                .offset(0)
                .size(64)])
        }
        .unwrap();

        let descriptor_pool = {
            let pool_sizes = [
                // 背景画像と文字描画で使うサンプラー
                // どっちもリニアでいいと思うので 1 つあればよい
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1),
                // 背景画像
                // グリフテクスチャー
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(2),
                // 適当な要素数を用意しておく
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(8),
                // 文字の描画で 1 つ使う
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1),
            ];
            let create_info = vk::DescriptorPoolCreateInfo::default()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            unsafe { device.create_descriptor_pool(&create_info, None) }.unwrap()
        };

        // 背景描画リソース
        let background_descriptor_set_layout = {
            let bindings = [
                // 頂点シェーダー
                // 画像の UV 合わせ
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                // ピクセルシェーダー
                // 背景画像
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            ];
            let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
            unsafe { device.create_descriptor_set_layout(&create_info, None) }.unwrap()
        };

        // 文字描画リソース
        let character_descriptor_set_layout = {
            let bindings = [
                // 文字の配置
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                // グリフテクスチャー
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            ];
            let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
            unsafe { device.create_descriptor_set_layout(&create_info, None) }.unwrap()
        };

        let descriptor_sets = {
            let set_layouts = [
                background_descriptor_set_layout,
                character_descriptor_set_layout,
            ];
            let allocate_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&set_layouts);
            unsafe { device.allocate_descriptor_sets(&allocate_info) }.unwrap()
        };

        {
            let buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .offset(0)
                .range(128)];
            let descriptor_writes = [vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[0])
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&buffer_info)];
            let descriptor_copies = [];
            unsafe { device.update_descriptor_sets(&descriptor_writes, &descriptor_copies) };
        }

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
                .command_buffer_count(1);
            unsafe { device.allocate_command_buffers(&allocate_info) }.unwrap()
        };

        let command_fence = {
            let create_info = vk::FenceCreateInfo::default();
            unsafe { device.create_fence(&create_info, None) }.unwrap()
        };

        let (display_semaphore, command_completed_semaphore) = {
            let create_info = vk::SemaphoreCreateInfo::default();
            let display_semaphore = unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            let command_completed_semaphore =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            (display_semaphore, command_completed_semaphore)
        };

        Self {
            instance,
            device,
            queue,
            render_pass,
            pipeline,
            command_fence,
            display_semaphore,
            command_completed_semaphore,
            surface,
            command_pool,
            command_buffer: command_buffers[0],
            surface_loader,
            swapchain_loader,
            swapchain,
            swapchain_images,
            present_image_views,
            framebuffers,

            // Resources
            buffer,
        }
    }

    pub async fn serve(self) {
        let device = &self.device;

        let (next_frame_index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.display_semaphore,
                vk::Fence::null(),
            )
        }
        .unwrap();

        let framebuffer = self.framebuffers[next_frame_index as usize];
        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(framebuffer)
            .render_area(
                vk::Rect2D::default().extent(vk::Extent2D::default().width(640).height(480)),
            )
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.1, 0.2, 0.3, 1.0],
                },
            }]);

        unsafe {
            device.reset_command_buffer(
                self.command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
        }
        .unwrap();

        {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            unsafe { device.begin_command_buffer(self.command_buffer, &begin_info) }.unwrap();
        }

        unsafe {
            device.cmd_begin_render_pass(
                self.command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::default(),
            )
        };

        unsafe {
            device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            )
        };

        unsafe {
            device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.layout,
                0, /*first_set*/
                &self.descriptor_sets,
                &[],
            )
        };

        unsafe {
            device.cmd_bind_vertex_buffers(
                self.command_buffer,
                0, /*first_binding*/
                &[self.buffer],
                &[0], /*offsets*/
            )
        };

        unsafe {
            device.cmd_bind_index_buffer(
                self.command_buffer,
                self.buffer,
                (std::mem::size_of::<f32>() * 16) as u64, /*offset*/
                vk::IndexType::UINT16,
            )
        };

        unsafe {
            device.cmd_draw_indexed(
                self.command_buffer,
                6, /*index_count*/
                1, /*instance_count*/
                0, /*first_index*/
                0, /*fertex_offset*/
                0, /*first_instance*/
            )
        };

        unsafe { device.cmd_end_render_pass(self.command_buffer) };

        unsafe { device.end_command_buffer(self.command_buffer) }.unwrap();

        {
            let wait_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [self.command_buffer];
            let wait_semaphores = [self.display_semaphore];
            let signal_semaphores = [self.command_completed_semaphore];
            let submit_info = [vk::SubmitInfo::default()
                .wait_dst_stage_mask(&wait_mask)
                .command_buffers(&command_buffers)
                .wait_semaphores(&wait_semaphores)
                .signal_semaphores(&signal_semaphores)];
            unsafe { device.queue_submit(self.queue, &submit_info, vk::Fence::null()) }.unwrap();
        }

        {
            let wait_semaphores = [self.command_completed_semaphore];
            let swapchain = [self.swapchain];
            let image_indices = [next_frame_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swapchain)
                .image_indices(&image_indices);
            unsafe {
                self.swapchain_loader
                    .queue_present(self.queue, &present_info)
            }
            .unwrap();
        }
    }

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
