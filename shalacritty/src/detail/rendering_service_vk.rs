use std::{borrow::Cow, ffi::c_void, io::Cursor};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use ash::*;

#[repr(C)]
struct BackgroundView {
    transform0: [f32; 4],
    transform1: [f32; 4],
}

#[repr(C)]
struct CharacterData {
    transform0: [f32; 4],
    transform1: [f32; 4],
    fg_color: [f32; 4],
    uv01: [f32; 4],
}

pub struct RenderingServiceVk {
    instance: ash::Instance,
    device: ash::Device,
    surface: vk::SurfaceKHR,
    queue: vk::Queue,
    debug_utils_loader: ext::debug_utils::Instance,
    debug_utils_messanger: vk::DebugUtilsMessengerEXT,

    dynamic_rendering_device: khr::dynamic_rendering::Device,

    pipelines: Vec<vk::Pipeline>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    // Graphics Framework
    surface_loader: khr::surface::Instance,
    swapchain_loader: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,
    command_fences: Vec<vk::Fence>,
    display_semaphores: Vec<vk::Semaphore>,
    command_completed_semaphores: Vec<vk::Semaphore>,

    // Resources
    image_memory: vk::DeviceMemory,
    buffer_memory: vk::DeviceMemory,
    buffer: vk::Buffer,
    pipeline_layouts: Vec<vk::PipelineLayout>,
    descriptor_sets: Vec<vk::DescriptorSet>,
    sampler: vk::Sampler,
    background_image: vk::Image,
    background_image_view: vk::ImageView,

    subpass_info_table: Vec<SubpassInfo>,

    // 再描画
    redraw_requested_receiver: tokio::sync::mpsc::Receiver<()>,
}

impl RenderingServiceVk {
    pub fn new<T>(window: T) -> (tokio::sync::mpsc::Sender<()>, Self)
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

        let shader_module = {
            let mut cursor = Cursor::new(include_bytes!("terminal.spv"));
            let code = ash::util::read_spv(&mut cursor).unwrap();
            let create_info = vk::ShaderModuleCreateInfo::default().code(&code);
            unsafe { device.create_shader_module(&create_info, None) }.unwrap()
        };

        let descriptor_pool = {
            // 要素数は適当に余裕を持たせておく
            let pool_sizes = [
                // 背景画像と文字描画で使うサンプラー
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLER)
                    .descriptor_count(4),
                // 背景画像
                // グリフテクスチャー
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(4),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(8),
                // 文字の描画で使う
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(4),
            ];
            let create_info = vk::DescriptorPoolCreateInfo::default()
                .max_sets(2)
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
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
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
                    .binding(4)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                // グリフテクスチャー
                vk::DescriptorSetLayoutBinding::default()
                    .binding(2)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(5)
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

        let background_layout = {
            let set_layouts = [background_descriptor_set_layout];
            let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
            unsafe { device.create_pipeline_layout(&create_info, None) }.unwrap()
        };
        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::default();
            unsafe { device.create_pipeline_layout(&create_info, None) }.unwrap()
        };
        let character_layout = {
            let set_layoutes = [character_descriptor_set_layout];
            let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layoutes);
            unsafe { device.create_pipeline_layout(&create_info, None) }.unwrap()
        };

        let pipelines = {
            let layout_table = [background_layout, layout, character_layout];
            let stage_table: [_; 3] = std::array::from_fn(|index| {
                let module_name_table = [
                    (c"background_vs", c"background_fs"),
                    (c"main_vs", c"main_fs"),
                    (c"character_vs", c"character_fs"),
                ];
                let (vs_name, fs_name) = module_name_table[index];
                [
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::VERTEX)
                        .module(shader_module)
                        .name(vs_name),
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::FRAGMENT)
                        .module(shader_module)
                        .name(fs_name),
                ]
            });

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
            let color_attachment_formats = [surface_format.format];
            let create_info = ash::vk::PipelineRenderingCreateInfo::default()
                .color_attachment_formats(&color_attachment_formats);
            let ptr =
                ((&create_info) as *const ash::vk::PipelineRenderingCreateInfo) as *const c_void;
            let create_infos: [_; 3] = std::array::from_fn(|index| {
                let mut info = ash::vk::GraphicsPipelineCreateInfo::default()
                    .stages(&stage_table[index])
                    .vertex_input_state(&vertex_input_state)
                    .input_assembly_state(&input_assembly_state)
                    .viewport_state(&viewport_state)
                    .rasterization_state(&rasterization_state)
                    .multisample_state(&multisample_state)
                    .depth_stencil_state(&depth_stencil_state)
                    .color_blend_state(&color_blend_state)
                    .dynamic_state(&dynamic_state)
                    .layout(layout_table[index]);
                info.p_next = ptr;
                info
            });

            unsafe {
                device.create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
            }
            .unwrap()
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
                        | vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::UNIFORM_BUFFER,
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

        let (vertex_buffer, index_buffer, background_view, character_data) = {
            let vertrex_ptr = unsafe {
                device.map_memory(
                    device_memory,
                    0, /*offset*/
                    16 * 1024,
                    vk::MemoryMapFlags::empty(),
                )
            }
            .unwrap() as *mut f32;
            let index_ptr = unsafe { vertrex_ptr.add(16) } as *mut u16;
            let background_view_ptr = unsafe { index_ptr.add(16) } as *mut BackgroundView;
            let character_data_ptr =
                unsafe { background_view_ptr.byte_add(64) } as *mut CharacterData;

            (
                unsafe { std::slice::from_raw_parts_mut(vertrex_ptr, 16) },
                unsafe { std::slice::from_raw_parts_mut(index_ptr, 16) },
                unsafe { background_view_ptr.as_mut().unwrap() },
                unsafe { std::slice::from_raw_parts_mut(character_data_ptr, 1024) },
            )
        };

        const VERTEX_DATA: [f32; 8] = [-0.5f32, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5];
        const INDEX_DATA: [u16; 6] = [0, 1, 2, 0, 2, 3];
        vertex_buffer[0..VERTEX_DATA.len()].copy_from_slice(&VERTEX_DATA);
        index_buffer[0..INDEX_DATA.len()].copy_from_slice(&INDEX_DATA);
        background_view.transform0 = [1.0, 0.0, 0.0, 1.0];
        background_view.transform1 = [0.0, 1.0, 0.0, 1.0];
        character_data[0].transform0 = [0.2, 0.0, -0.5, 0.0];
        character_data[0].transform1 = [0.0, 0.2, -0.5, 0.0];
        character_data[1].transform0 = [0.2, 0.0, 0.5, 0.0];
        character_data[1].transform1 = [0.0, 0.2, 0.5, 0.0];

        unsafe {
            device.flush_mapped_memory_ranges(&[vk::MappedMemoryRange::default()
                .memory(device_memory)
                .offset(0)
                .size(1024)])
        }
        .unwrap();

        let image = {
            let create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(vk::Extent3D::default().width(128).height(128).depth(1))
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .queue_family_indices(&[0])
                .initial_layout(vk::ImageLayout::UNDEFINED);
            unsafe { device.create_image(&create_info, None) }.unwrap()
        };

        let image_memory = {
            let memory_index = {
                let memory_requirement = unsafe { device.get_image_memory_requirements(image) };
                unsafe { instance.get_physical_device_memory_properties(physical_device) }
                    .memory_types_as_slice()
                    .iter()
                    .enumerate()
                    .find(|(index, memory_type)| {
                        let flags = vk::MemoryPropertyFlags::DEVICE_LOCAL;
                        (1 << index) & memory_requirement.memory_type_bits != 0
                            && memory_type.property_flags & flags == flags
                    })
                    .map(|(index, _)| index as u32)
                    .unwrap()
            };
            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(16 * 512 * 512)
                .memory_type_index(memory_index);
            let device_memory = unsafe { device.allocate_memory(&allocate_info, None) }.unwrap();
            unsafe { device.bind_image_memory(image, device_memory, 0) }.unwrap();
            device_memory
        };

        let image_view = {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .components(
                    vk::ComponentMapping::default()
                        .r(vk::ComponentSwizzle::R)
                        .g(vk::ComponentSwizzle::G)
                        .b(vk::ComponentSwizzle::B)
                        .a(vk::ComponentSwizzle::A),
                )
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .base_mip_level(0)
                        .level_count(1)
                        .layer_count(1)
                        .base_array_layer(0)
                        .aspect_mask(vk::ImageAspectFlags::COLOR),
                );
            unsafe { device.create_image_view(&create_info, None) }.unwrap()
        };

        let sampler = {
            let create_info = vk::SamplerCreateInfo::default();
            unsafe { device.create_sampler(&create_info, None) }.unwrap()
        };

        {
            // 64 byte 分なアラインメント調節
            // TODO: アラインメントはデバイスに問い合わせた値を利用する
            let buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .offset(4 * 16 + 64)
                .range(std::mem::size_of::<BackgroundView>() as u64)];
            let character_buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .offset(4 * 16 + 64 + std::mem::size_of::<BackgroundView>() as u64)
                .range(2 * std::mem::size_of::<CharacterData>() as u64)];
            let image_info = [vk::DescriptorImageInfo::default()
                .sampler(sampler)
                .image_view(image_view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
            let descriptor_writes = [
                // 背景描画用
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&buffer_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[0])
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&image_info),
                // 文字描画用
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[1])
                    .dst_binding(4)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&character_buffer_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[1])
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&image_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[1])
                    .dst_binding(5)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
            ];
            unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };
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

        let (redraw_requested_sender, redraw_requested_receiver) = tokio::sync::mpsc::channel(1);

        (
            redraw_requested_sender,
            Self {
                instance,
                device,
                queue,
                debug_utils_loader,
                debug_utils_messanger: debug_utils,
                dynamic_rendering_device,
                pipelines,
                command_fences,
                display_semaphores,
                command_completed_semaphores,
                surface,
                command_pool,
                command_buffers,
                surface_loader,
                swapchain_loader,
                swapchain,
                swapchain_images,
                present_image_views,
                redraw_requested_receiver,
                // Resources
                buffer_memory: device_memory,
                image_memory,
                buffer,
                pipeline_layouts: vec![background_layout, layout, character_layout],
                descriptor_sets,
                sampler,
                background_image: image,
                background_image_view: image_view,

                subpass_info_table: vec![
                    // 背景
                    SubpassInfo {
                        pipeline_index: 0,
                        instance_count: 1,
                        descriptor_set_index: Some(0),
                        pipeline_layout_index: Some(0),
                    },
                    // デバッグ
                    SubpassInfo {
                        pipeline_index: 1,
                        instance_count: 1,
                        descriptor_set_index: None,
                        pipeline_layout_index: None,
                    },
                    // 文字
                    SubpassInfo {
                        pipeline_index: 2,
                        instance_count: 2, // TODO
                        descriptor_set_index: Some(1),
                        pipeline_layout_index: Some(2),
                    },
                ],
            },
        )
    }

    pub async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        let mut draw_context = DrawContext {
            frame: 0,
            is_background_updated: false,
        };
        loop {
            tokio::select! {
                Some(_) = self.redraw_requested_receiver.recv() => self.draw(&mut draw_context).await,
                _ = &mut cancellation_token => {
                    unsafe { self.device.queue_wait_idle(self.queue) }.unwrap();
                    break;
                },
                else => {}
            }
        }
    }

    async fn draw(&self, context: &mut DrawContext) {
        // スコープを抜けるときにフレーム数を増やす
        struct Exit<'a> {
            context: &'a mut DrawContext,
        }
        impl<'a> Drop for Exit<'a> {
            fn drop(&mut self) {
                self.context.frame += 1;
                self.context.is_background_updated = true;
            }
        }
        let is_background_updated = true;
        let frame = context.frame;
        let next_frame = (frame % 2) as usize;
        #[allow(unused)]
        let exit = Exit { context };

        let device = &self.device;
        let display_semaphore = self.display_semaphores[next_frame];
        let command_completed_semaphore = self.command_completed_semaphores[next_frame];
        let command_buffer = self.command_buffers[next_frame];
        let previous_command_fence = self.command_fences[(next_frame + 1) % 2];
        let next_command_fence = self.command_fences[next_frame];

        let (next_frame_index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                display_semaphore,
                vk::Fence::null(),
            )
        }
        .unwrap();

        if frame > 0 {
            unsafe { device.wait_for_fences(&[previous_command_fence], true, u64::MAX) }.unwrap();
        }
        unsafe { device.reset_fences(&[next_command_fence]) }.unwrap();

        unsafe {
            device.reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
        }
        .unwrap();

        {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            unsafe { device.begin_command_buffer(command_buffer, &begin_info) }.unwrap();
        }

        if is_background_updated {
            let texture_barrier = vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(self.background_image)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .level_count(1)
                        .layer_count(1),
                );
            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier],
                )
            };
        }

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

        let color_attachments = [ash::vk::RenderingAttachmentInfo::default()
            .image_view(self.present_image_views[next_frame_index as usize])
            .image_layout(ash::vk::ImageLayout::ATTACHMENT_OPTIMAL)
            .load_op(ash::vk::AttachmentLoadOp::CLEAR)
            .clear_value(ash::vk::ClearValue {
                color: ash::vk::ClearColorValue {
                    float32: [0.1, 0.2, 0.3, 1.0],
                },
            })];
        let begin_info = ash::vk::RenderingInfo::default()
            .render_area(
                ash::vk::Rect2D::default()
                    .extent(ash::vk::Extent2D::default().width(640).height(480)),
            )
            .layer_count(1)
            .color_attachments(&color_attachments);
        unsafe {
            self.dynamic_rendering_device
                .cmd_begin_rendering(command_buffer, &begin_info)
        };

        for subpass_info in &self.subpass_info_table {
            unsafe {
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipelines[subpass_info.pipeline_index],
                )
            };

            unsafe {
                device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0, /*first_binding*/
                    &[self.buffer],
                    &[0], /*offsets*/
                )
            };

            if let Some((layout_index, set_index)) = match (
                subpass_info.pipeline_layout_index,
                subpass_info.descriptor_set_index,
            ) {
                (Some(layout_index), Some(descriptor_set_index)) => {
                    Some((layout_index, descriptor_set_index))
                }
                _ => None,
            } {
                let layout = self.pipeline_layouts[layout_index];
                let descriptor_set = self.descriptor_sets[set_index];

                unsafe {
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        layout,
                        0, /*first_set*/
                        &[descriptor_set],
                        &[],
                    )
                }
            }

            unsafe {
                device.cmd_bind_index_buffer(
                    command_buffer,
                    self.buffer,
                    (std::mem::size_of::<f32>() * 16) as u64, /*offset*/
                    vk::IndexType::UINT16,
                )
            };

            unsafe {
                device.cmd_draw_indexed(
                    command_buffer,
                    6,                           /*index_count*/
                    subpass_info.instance_count, /*instance_count*/
                    0,                           /*first_index*/
                    0,                           /*fertex_offset*/
                    0,                           /*first_instance*/
                )
            };
        }

        unsafe {
            self.dynamic_rendering_device
                .cmd_end_rendering(command_buffer)
        };

        unsafe {
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
            )
        };

        unsafe { device.end_command_buffer(command_buffer) }.unwrap();

        {
            let wait_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [command_buffer];
            let wait_semaphores = [display_semaphore];
            let signal_semaphores = [command_completed_semaphore];
            let submit_info = [vk::SubmitInfo::default()
                .wait_dst_stage_mask(&wait_mask)
                .command_buffers(&command_buffers)
                .wait_semaphores(&wait_semaphores)
                .signal_semaphores(&signal_semaphores)];
            unsafe { device.queue_submit(self.queue, &submit_info, next_command_fence) }.unwrap();
        }

        {
            let wait_semaphores = [command_completed_semaphore];
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

impl Drop for RenderingServiceVk {
    fn drop(&mut self) {
        let device = &self.device;

        unsafe { device.destroy_image(self.background_image, None) };
        unsafe { device.destroy_image_view(self.background_image_view, None) };
        unsafe { device.free_memory(self.image_memory, None) };
        unsafe { device.destroy_sampler(self.sampler, None) };

        unsafe { device.destroy_buffer(self.buffer, None) };
        unsafe { device.free_memory(self.buffer_memory, None) };

        for semaphore in &self.command_completed_semaphores {
            unsafe { device.destroy_semaphore(*semaphore, None) };
        }

        for semaphore in &self.display_semaphores {
            unsafe { device.destroy_semaphore(*semaphore, None) };
        }

        for fence in &self.command_fences {
            unsafe { device.destroy_fence(*fence, None) };
        }

        unsafe { device.free_command_buffers(self.command_pool, &self.command_buffers) };
        unsafe { device.destroy_command_pool(self.command_pool, None) };

        for pipeline in &self.pipelines {
            unsafe { device.destroy_pipeline(*pipeline, None) };
        }

        for image in &self.swapchain_images {
            unsafe { device.destroy_image(*image, None) };
        }

        for image_voew in &self.present_image_views {
            unsafe { device.destroy_image_view(*image_voew, None) };
        }

        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };

        unsafe { self.surface_loader.destroy_surface(self.surface, None) };

        unsafe {
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_utils_messanger, None);
        }
        unsafe { device.destroy_device(None) };
        unsafe { self.instance.destroy_instance(None) };
    }
}

impl renge::Service for RenderingServiceVk {
    fn serve(
        self,
        cancellation_token: renge::CancellationToken,
    ) -> impl std::prelude::rust_2024::Future<Output = ()> + Send {
        self.serve(cancellation_token)
    }
}

struct DrawContext {
    frame: u64,
    is_background_updated: bool,
}

struct SubpassInfo {
    pipeline_index: usize,

    instance_count: u32,

    descriptor_set_index: Option<usize>,

    pipeline_layout_index: Option<usize>,
}
