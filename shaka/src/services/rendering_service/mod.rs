mod buffer_layout;
mod buffer_view;
mod glyph_table;
mod range_allocator;
mod text_writer;
mod transfer_queue;
use glyph_table::GlyphTable;
use range_allocator::RangeAllocator;

use std::{borrow::Cow, io::Cursor, mem::offset_of, u64};

use ash::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::services::{
    glyph_extract_service::{FontId, GlyphRequest},
    rendering_service::{
        buffer_layout::BufferLayout,
        buffer_view::CharacterData,
        text_writer::{CopyRange, TextWriter},
        transfer_queue::TransferQueue,
    },
    shell_service::{Patch, TextData},
};

pub struct RenderingServiceParams {
    pub receiver: tokio::sync::mpsc::Receiver<()>,
    pub content_receiver: tokio::sync::mpsc::Receiver<TextData>,
    pub glyph_request_sender: tokio::sync::mpsc::Sender<GlyphRequest>,
}

struct DrawParams {
    char_count: u32,
    frame: u64,
    image_layout: vk::ImageLayout,
    transfer_queue: TransferQueue<CharacterData>,
}

pub struct RenderingService {
    glyph_table: GlyphTable,
    text_writer: TextWriter,
    range_allocator: RangeAllocator,

    instance: ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    #[allow(unused)]
    queue: vk::Queue,
    debug_utils_loader: ext::debug_utils::Instance,
    debug_utils_messanger: vk::DebugUtilsMessengerEXT,

    dynamic_rendering_device: khr::dynamic_rendering::Device,

    // Graphics Framework
    image_count: usize,
    surface: vk::SurfaceKHR,
    surface_loader: khr::surface::Instance,
    swapchain_loader: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    #[allow(unused)]
    swapchain_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,

    // フレーム同期
    display_semaphores: Vec<vk::Semaphore>,
    command_completed_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    // コマンド同期用のタイムラインセマフォ
    command_semaphore: vk::Semaphore,

    command_pool: vk::CommandPool,
    // 0, 1 →  描画コマンドのダブルバッファリング
    // 2 -> コピー用のコマンドバッファー
    command_buffers: Vec<vk::CommandBuffer>,

    // パイプライン関係
    shader_module: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    pipelines: Vec<vk::Pipeline>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    descriptor_set_layout: vk::DescriptorSetLayout,

    // リソース
    buffer_memory: vk::DeviceMemory,
    copy_src_memory: vk::DeviceMemory,
    buffer: vk::Buffer,
    buffer_layout: BufferLayout,
    vertex_data_index: usize,
    index_data_index: usize,
    character_data_index: usize,
    copy_src_buffer: vk::Buffer,

    // グリフ
    glyph_image: vk::Image,
    glyph_image_view: vk::ImageView,
    glyph_memory: vk::DeviceMemory,
    glyph_sampler: vk::Sampler,
}

impl RenderingService {
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
            let mut timeline_semaphore_features =
                vk::PhysicalDeviceTimelineSemaphoreFeatures::default().timeline_semaphore(true);
            let device_create_info = ash::vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features)
                .push_next(&mut vulkan_features)
                .push_next(&mut dynamic_rendering_features)
                .push_next(&mut sync_features)
                .push_next(&mut timeline_semaphore_features);
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

        let shader_module = {
            let mut cursor = Cursor::new(include_bytes!("terminal.spv"));
            let code = ash::util::read_spv(&mut cursor).unwrap();
            let create_info = vk::ShaderModuleCreateInfo::default().code(&code);
            unsafe { device.create_shader_module(&create_info, None) }.unwrap()
        };

        let descriptor_pool = {
            let pool_sizes = [
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(8),
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::SAMPLER)
                    .descriptor_count(8),
            ];
            let create_info = vk::DescriptorPoolCreateInfo::default()
                .max_sets(1)
                .pool_sizes(&pool_sizes);
            unsafe { device.create_descriptor_pool(&create_info, None) }.unwrap()
        };

        let descriptor_set_layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            ];
            let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
            unsafe { device.create_descriptor_set_layout(&create_info, None) }.unwrap()
        };

        let descriptor_sets = {
            let set_layouts = [descriptor_set_layout];
            let allocate_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&set_layouts);
            unsafe { device.allocate_descriptor_sets(&allocate_info) }.unwrap()
        };

        let layout = {
            let set_layouts = [descriptor_set_layout];
            let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
            unsafe { device.create_pipeline_layout(&create_info, None) }.unwrap()
        };

        let pipelines = {
            let stages = [
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(shader_module)
                    .name(c"character_vs_tentative"),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(shader_module)
                    .name(c"character_fs_tentative"),
            ];
            // 頂点データは全インスタンスで共通だが、
            // 文字ごとのデータはインスタンスごとのデータなのでそれぞれバッファーを分ける
            let vertex_binding_descriptions = [
                vk::VertexInputBindingDescription::default()
                    .binding(0)
                    .stride(std::mem::size_of::<f32>() as u32 * 2)
                    .input_rate(vk::VertexInputRate::VERTEX),
                vk::VertexInputBindingDescription::default()
                    .binding(1)
                    .stride(std::mem::size_of::<CharacterData>() as u32)
                    .input_rate(vk::VertexInputRate::INSTANCE),
            ];
            let vertex_attribute_descriptions = [
                vk::VertexInputAttributeDescription::default()
                    .binding(0)
                    .location(0)
                    .format(vk::Format::R32G32_SFLOAT)
                    .offset(0),
                vk::VertexInputAttributeDescription::default()
                    .binding(1)
                    .location(1)
                    .format(vk::Format::R32G32B32A32_SFLOAT)
                    .offset(offset_of!(CharacterData, transform0) as u32),
                vk::VertexInputAttributeDescription::default()
                    .binding(1)
                    .location(2)
                    .format(vk::Format::R32G32B32A32_SFLOAT)
                    .offset(offset_of!(CharacterData, transform1) as u32),
                vk::VertexInputAttributeDescription::default()
                    .binding(1)
                    .location(3)
                    .format(vk::Format::R32G32B32A32_SFLOAT)
                    .offset(offset_of!(CharacterData, fg_color) as u32),
                vk::VertexInputAttributeDescription::default()
                    .binding(1)
                    .location(4)
                    .format(vk::Format::R32G32B32A32_SFLOAT)
                    .offset(offset_of!(CharacterData, uv01) as u32),
            ];
            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&vertex_binding_descriptions)
                .vertex_attribute_descriptions(&vertex_attribute_descriptions);
            let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let viewpors = [vk::Viewport::default().width(1280.0).height(960.0)];
            let scissors =
                [vk::Rect2D::default().extent(vk::Extent2D::default().width(1280).height(960))];
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
            let mut create_info = ash::vk::PipelineRenderingCreateInfo::default()
                .color_attachment_formats(&color_attachment_formats);
            let create_infos = [ash::vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_state)
                .depth_stencil_state(&depth_stencil_state)
                .color_blend_state(&color_blend_state)
                .dynamic_state(&dynamic_state)
                .layout(layout)
                .push_next(&mut create_info)];

            unsafe {
                device.create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
            }
            .unwrap()
        };

        const COPY_SRC_BUFFER_SIZE: vk::DeviceSize = 64 * 1024;
        let copy_src_buffer = {
            let queue_family_indices = [queue_family_index as u32];
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(&queue_family_indices)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .size(COPY_SRC_BUFFER_SIZE)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC);
            unsafe { device.create_buffer(&create_info, None) }.unwrap()
        };

        const BUFFER_SIZE: vk::DeviceSize = 128 * 1024;
        let buffer = {
            let queue_family_indices = [queue_family_index as u32];
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(&queue_family_indices)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .size(BUFFER_SIZE)
                .usage(
                    vk::BufferUsageFlags::VERTEX_BUFFER
                        | vk::BufferUsageFlags::INDEX_BUFFER
                        | vk::BufferUsageFlags::UNIFORM_BUFFER
                        | vk::BufferUsageFlags::TRANSFER_DST,
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

        let copy_src_memory = {
            let memory_index = {
                let memory_requirement =
                    unsafe { device.get_buffer_memory_requirements(copy_src_buffer) };
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
                .allocation_size(COPY_SRC_BUFFER_SIZE)
                .memory_type_index(memory_index);
            unsafe { device.allocate_memory(&allocate_info, None) }.unwrap()
        };

        unsafe { device.bind_buffer_memory(copy_src_buffer, copy_src_memory, 0) }.unwrap();

        let ptr = unsafe {
            device.map_memory(
                device_memory,
                0, /*offset*/
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
        }
        .unwrap();

        let limits = unsafe {
            &instance
                .get_physical_device_properties(physical_device)
                .limits
        };
        let min_storage_buffer_offset_alignment =
            limits.min_storage_buffer_offset_alignment as usize;
        // let min_uniform_buffer_offset_alignment =
        //     limits.min_uniform_buffer_offset_alignment as usize;
        // let non_coherent_atom_size = limits.non_coherent_atom_size as usize;

        let mut buffer_layout = BufferLayout::new();
        let vertex_data_alignment =
            min_storage_buffer_offset_alignment.max(std::mem::align_of::<f32>());
        let vertex_data_index = buffer_layout.add::<f32>(8, vertex_data_alignment);
        let index_data_alignment =
            min_storage_buffer_offset_alignment.max(std::mem::align_of::<u16>());
        let index_data_index = buffer_layout.add::<u16>(6, index_data_alignment);
        let character_data_alignment =
            min_storage_buffer_offset_alignment.max(std::mem::align_of::<CharacterData>());
        let character_data_index =
            buffer_layout.add::<CharacterData>(1024, character_data_alignment);

        {
            const VERTEX_DATA: [f32; 8] = [-0.5f32, 0.5, -0.5, -0.5, 0.5, 0.5, 0.5, -0.5];
            buffer_layout
                .get_slice_mut(ptr as *mut f32, vertex_data_index)
                .copy_from_slice(&VERTEX_DATA);
        }
        {
            const INDEX_DATA: [u16; 6] = [0, 1, 2, 2, 1, 3];
            buffer_layout
                .get_slice_mut(ptr as *mut u16, index_data_index)
                .copy_from_slice(&INDEX_DATA);
        }

        // background_view.transform0 = [1.0, 0.0, 0.0, 1.0];
        // background_view.transform1 = [0.0, 1.0, 0.0, 1.0];

        unsafe {
            let atom_size = limits.non_coherent_atom_size as usize;
            let total_size = buffer_layout.total_size();
            let flush_size = if total_size % atom_size == 0 {
                total_size
            } else {
                total_size + (atom_size - (total_size % atom_size))
            };
            device.flush_mapped_memory_ranges(&[vk::MappedMemoryRange::default()
                .memory(device_memory)
                .offset(0)
                .size(flush_size as vk::DeviceSize)])
        }
        .unwrap();

        let glyph_image = {
            let create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8_UNORM)
                .extent(vk::Extent3D::default().width(4096).height(4096).depth(1))
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .tiling(vk::ImageTiling::OPTIMAL)
                .samples(vk::SampleCountFlags::TYPE_1)
                .array_layers(1)
                .mip_levels(1)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            unsafe { device.create_image(&create_info, None) }.unwrap()
        };

        let glyph_memory = {
            let requirements = unsafe { device.get_image_memory_requirements(glyph_image) };
            let memory_index = {
                unsafe { instance.get_physical_device_memory_properties(physical_device) }
                    .memory_types_as_slice()
                    .iter()
                    .enumerate()
                    .find(|(index, memory_type)| {
                        let flags = vk::MemoryPropertyFlags::DEVICE_LOCAL;
                        (1 << index) & requirements.memory_type_bits != 0
                            && memory_type.property_flags & flags == flags
                    })
                    .map(|(index, _)| index as u32)
                    .unwrap()
            };
            let create_info = vk::MemoryAllocateInfo::default()
                .allocation_size(requirements.size)
                .memory_type_index(memory_index);
            unsafe { device.allocate_memory(&create_info, None) }.unwrap()
        };

        unsafe {
            device
                .bind_image_memory(glyph_image, glyph_memory, 0)
                .unwrap();
        }

        let glyph_image_view = {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(glyph_image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8_UNORM)
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
            unsafe { device.create_image_view(&create_info, None) }.unwrap()
        };

        let sampler = {
            let create_info = vk::SamplerCreateInfo::default();
            unsafe { device.create_sampler(&create_info, None) }.unwrap()
        };

        unsafe {
            device.update_descriptor_sets(
                &[
                    vk::WriteDescriptorSet::default()
                        .dst_set(descriptor_sets[0])
                        .dst_binding(0)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::SAMPLER)
                        .image_info(&[vk::DescriptorImageInfo::default().sampler(sampler)]),
                    vk::WriteDescriptorSet::default()
                        .dst_set(descriptor_sets[0])
                        .dst_binding(1)
                        .dst_array_element(0)
                        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                        .image_info(&[vk::DescriptorImageInfo::default()
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                            .image_view(glyph_image_view)]),
                ],
                &[],
            )
        };

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
                .command_buffer_count((swapchain_images.len() * 2) as u32);
            unsafe { device.allocate_command_buffers(&allocate_info) }.unwrap()
        };

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
            glyph_table: GlyphTable::new(4096, 4096),
            text_writer: TextWriter::new(),
            range_allocator: RangeAllocator::new(4096, 4096),
            instance,
            device,
            physical_device,
            debug_utils_loader,
            debug_utils_messanger: debug_utils,
            dynamic_rendering_device,
            display_semaphores,
            command_completed_semaphores,
            command_semaphore,
            in_flight_fences,
            command_pool,
            command_buffers,
            image_count: swapchain_images.len(),
            surface,
            queue,
            surface_loader,
            swapchain_loader,
            swapchain,
            swapchain_images,
            present_image_views,

            // 描画
            shader_module,
            pipeline_layout: layout,
            pipelines,
            descriptor_pool,
            descriptor_sets,
            descriptor_set_layout,
            buffer_memory: device_memory,
            copy_src_memory,
            buffer,
            buffer_layout,
            vertex_data_index,
            index_data_index,
            character_data_index,

            copy_src_buffer,
            // グリフ
            glyph_image,
            glyph_image_view,
            glyph_memory,
            glyph_sampler: sampler,
        }
    }

    async fn serve(
        mut self,
        mut params: RenderingServiceParams,
        mut cancellation_token: renge::CancellationToken,
    ) {
        let mut receiver = params.receiver;
        let mut content_receiver = params.content_receiver;

        let mut draw_params = DrawParams {
            char_count: 64,
            frame: 0,
            image_layout: vk::ImageLayout::UNDEFINED,
            transfer_queue: TransferQueue::new(),
        };
        loop {
            tokio::select! {
                Some(()) = receiver.recv() => {
                    self.draw(&draw_params);
                    draw_params.frame += 1;
                    draw_params.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                },
                Some(text_data) = content_receiver.recv() => {
                    draw_params.char_count = text_data.char_count as u32;
                    // 内部で draw を呼び出します
                    self.apply_patch(&draw_params, &text_data.patches, &mut params.glyph_request_sender)
                        .await;
                    draw_params.frame += 1;
                    draw_params.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                },
                _ = &mut cancellation_token => break,
                else => {},
            }
        }
    }

    async fn apply_patch(
        &mut self,
        params: &DrawParams,
        patches: &[Patch],
        sender: &tokio::sync::mpsc::Sender<GlyphRequest>,
    ) {
        let device = &self.device;

        // 文字ごとの情報を転送するコマンド
        unsafe {
            // ラスタライズで await をまたいで Map した生ポインターにアクセスするとコンパイルエラーになる
            // そこで一時的なバッファーに書き出しておいて後で Map したメモリーにコピーする手法を採用している
            let mut dst_buffer = Vec::with_capacity(1024);
            let mut buffer_image_copies = Vec::default();
            for code in patches.iter().map(|c| c.content.code) {
                let (s, receiver) = tokio::sync::oneshot::channel();
                sender
                    .send(GlyphRequest {
                        code,
                        font_id: FontId::default(),
                        size: 32.0f32,
                        response: s,
                    })
                    .await
                    .unwrap();

                let Ok(mut glyph) = receiver.await else {
                    continue;
                };

                // 管理用データを構築
                let Some(offset) = self
                    .range_allocator
                    .allocate(glyph.width as u32, glyph.height as u32)
                else {
                    continue;
                };

                self.glyph_table.insert_glyph(
                    glyph.code,
                    offset[0],
                    offset[1],
                    glyph.width as u32,
                    glyph.height as u32,
                );

                // グリフデータの書き込み
                let Some(rect) = self.glyph_table.get_rect(code) else {
                    continue;
                };
                let buffer_image_copy = vk::BufferImageCopy::default()
                    .buffer_offset(dst_buffer.len() as u64)
                    .buffer_image_height(0)
                    .buffer_row_length(0)
                    .image_offset(
                        vk::Offset3D::default()
                            .x(rect.offsetx as i32)
                            .y(rect.offsety as i32),
                    )
                    .image_extent(
                        vk::Extent3D::default()
                            .width(rect.width)
                            .height(rect.height)
                            .depth(1),
                    )
                    .image_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .mip_level(0)
                            .base_array_layer(0)
                            .layer_count(1),
                    );
                buffer_image_copies.push(buffer_image_copy);
                dst_buffer.append(&mut glyph.data);
            }

            let limits = &self
                .instance
                .get_physical_device_properties(self.physical_device)
                .limits;
            let mut buffer_layout = BufferLayout::new();
            let glyph_data_index = buffer_layout.add::<u8>(dst_buffer.len(), 1);
            let character_data_index = buffer_layout
                .add::<CharacterData>(128, limits.min_storage_buffer_offset_alignment as usize);

            let glyph_section = buffer_layout.get_section(glyph_data_index);
            for copy in &mut buffer_image_copies {
                copy.buffer_offset += glyph_section.offset as u64;
            }

            let ptr = device
                .map_memory(
                    self.copy_src_memory,
                    0,                                            /*offset*/
                    buffer_layout.total_size() as vk::DeviceSize, /*size*/
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap() as *mut u8;

            // フォントデータのコピー
            buffer_layout
                .get_slice_mut(ptr as *mut u8, glyph_data_index)
                .copy_from_slice(&dst_buffer);

            let character_data =
                buffer_layout.get_slice_mut(ptr as *mut CharacterData, character_data_index);

            let copy_ranges: Vec<vk::BufferCopy> =
                self.text_writer
                    .write(character_data, patches, &self.glyph_table);
            buffer_layout.resize::<CharacterData>(character_data_index, copy_ranges.len());

            // フラッシュは特定の値の倍数である必要がある
            let atom_size = limits.non_coherent_atom_size as usize;
            let total_size = buffer_layout.total_size();
            let flush_size = if total_size % atom_size == 0 {
                total_size
            } else {
                total_size + (atom_size - (total_size % atom_size))
            };
            let ranges = [vk::MappedMemoryRange::default()
                .memory(self.copy_src_memory)
                .offset(0)
                .size(flush_size as vk::DeviceSize)];
            device.flush_mapped_memory_ranges(&ranges).unwrap();
            device.unmap_memory(self.copy_src_memory);

            self.draw_with_callback(params, |command_buffer, _params| {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                    ash::vk::PipelineStageFlags::TRANSFER,
                    ash::vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[ash::vk::ImageMemoryBarrier::default()
                        .old_layout(params.image_layout)
                        .new_layout(ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .image(self.glyph_image)
                        .subresource_range(
                            ash::vk::ImageSubresourceRange::default()
                                .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(1)
                                .base_array_layer(0)
                                .layer_count(1),
                        )
                        .src_access_mask(ash::vk::AccessFlags::NONE)
                        .dst_access_mask(ash::vk::AccessFlags::TRANSFER_WRITE)],
                );

                if !buffer_image_copies.is_empty() {
                    device.cmd_copy_buffer_to_image(
                        command_buffer,
                        self.copy_src_buffer,
                        self.glyph_image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &buffer_image_copies,
                    );
                }

                if !copy_ranges.is_empty() {
                    let src_char_section = buffer_layout.get_section(character_data_index);
                    let dst_char_section =
                        self.buffer_layout.get_section(self.character_data_index);
                    let regions: Vec<_> = copy_ranges
                        .iter()
                        .map(|x| {
                            x.src_offset(x.src_offset + src_char_section.offset as u64)
                                .dst_offset(x.dst_offset + dst_char_section.offset as u64)
                        })
                        .collect();
                    device.cmd_copy_buffer(
                        command_buffer,
                        self.copy_src_buffer,
                        self.buffer,
                        &regions,
                    );

                    // バッファーのコピー待ち
                    let buffer_barrier = vk::BufferMemoryBarrier2::default()
                        // コピー操作（TRANSFERステージでの書き込み）の完了を待つ
                        .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                        // 頂点入力（VERTEX_INPUTステージでの読み込み）の前に完了を保証する
                        .dst_stage_mask(vk::PipelineStageFlags2::VERTEX_INPUT)
                        .dst_access_mask(vk::AccessFlags2::VERTEX_ATTRIBUTE_READ)
                        .buffer(self.buffer)
                        .offset(dst_char_section.offset as vk::DeviceSize)
                        .size(dst_char_section.size as vk::DeviceSize);
                    let dependency_info = vk::DependencyInfo::default()
                        .buffer_memory_barriers(std::slice::from_ref(&buffer_barrier));
                    device.cmd_pipeline_barrier2(command_buffer, &dependency_info);
                }

                device.cmd_pipeline_barrier(
                    command_buffer,
                    ash::vk::PipelineStageFlags::TRANSFER,
                    ash::vk::PipelineStageFlags::FRAGMENT_SHADER,
                    ash::vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[ash::vk::ImageMemoryBarrier::default()
                        .old_layout(ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image(self.glyph_image)
                        .subresource_range(
                            ash::vk::ImageSubresourceRange::default()
                                .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(1)
                                .base_array_layer(0)
                                .layer_count(1),
                        )
                        .src_access_mask(ash::vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(ash::vk::AccessFlags::SHADER_READ)],
                );
                Some(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            });
        }
    }

    fn draw(&self, params: &DrawParams) {
        self.draw_with_callback(params, |_, _| None);
    }

    fn draw_with_callback<F>(&self, params: &DrawParams, callback: F)
    where
        F: FnOnce(vk::CommandBuffer, &DrawParams) -> Option<vk::ImageLayout>,
    {
        if params.char_count == 0 {
            return;
        }

        let buffer_index = (params.frame as usize) % self.image_count;
        let device = &self.device;
        let display_semaphore = self.display_semaphores[buffer_index];
        let in_flight_fence = self.in_flight_fences[buffer_index];
        let command_buffer = self.command_buffers[buffer_index];

        // コマンドバッファーが空いているか
        unsafe {
            device
                .wait_for_fences(&[in_flight_fence], true, u64::MAX)
                .unwrap()
        }

        let (next_frame_index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                display_semaphore,
                vk::Fence::null(),
            )
        }
        .unwrap();

        let command_completed_semaphore =
            self.command_completed_semaphores[next_frame_index as usize];

        // フェンスをリセット
        // ウィンドウのリサイズなどでイメージの取得に失敗することがあるので、
        // ここで待つことで確実に描画を開始できる状態であることを保証できる
        unsafe { device.reset_fences(&[in_flight_fence]).unwrap() };

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

        // コールバックで遷移が発生するか、すでに遷移済みじゃなかったら遷移する
        let new_glyph_layout = callback(command_buffer, params);
        if new_glyph_layout.is_some() {
            // いい感じに遷移してくれてるのでなにもしない
        } else if params.image_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
            // すでに遷移済みなので何もしない
        } else {
            // 誰も遷移させてないので UNDEFINED のまま
            // シェーダーで読み取れるように遷移させる
            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                    ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    ash::vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[ash::vk::ImageMemoryBarrier::default()
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image(self.glyph_image)
                        .subresource_range(
                            ash::vk::ImageSubresourceRange::default()
                                .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(1)
                                .base_array_layer(0)
                                .layer_count(1),
                        )],
                )
            }
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

        let color_attachments = [vk::RenderingAttachmentInfo::default()
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
                    .extent(ash::vk::Extent2D::default().width(1280).height(960)),
            )
            .layer_count(1)
            .color_attachments(&color_attachments);
        unsafe {
            self.dynamic_rendering_device
                .cmd_begin_rendering(command_buffer, &begin_info);

            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipelines[0],
            );

            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0, /*first_set*/
                &self.descriptor_sets,
                &[],
            );

            // ひとつのバッファーを分割してふたつの頂点データとして利用
            // 矩形をシェーダー上で生成してしまえば頂点分のデータはいらなくなるかも
            let vertex_section = self.buffer_layout.get_section(self.vertex_data_index);
            let character_data_section = self.buffer_layout.get_section(self.character_data_index);
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0, /*first_binding*/
                &[self.buffer, self.buffer],
                &[
                    vertex_section.offset as u64,
                    character_data_section.offset as u64,
                ],
            );

            let index_section = self.buffer_layout.get_section(self.index_data_index);
            device.cmd_bind_index_buffer(
                command_buffer,
                self.buffer,
                index_section.offset as u64, /*offset*/
                vk::IndexType::UINT16,
            );

            device.cmd_draw_indexed(
                command_buffer,
                6,                 /*index_count*/
                params.char_count, /*instance_count*/
                0,                 /*first_index*/
                0,                 /*vertex_offset*/
                0,                 /*first_instance*/
            );

            self.dynamic_rendering_device
                .cmd_end_rendering(command_buffer)
        }

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

        let command_buffer_infos =
            [vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer)];
        let wait_semaphore_infos = [vk::SemaphoreSubmitInfo::default()
            .semaphore(display_semaphore)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)];
        let signal_semaphore_infos = [
            // コマンドバッファー同期用のセマフォの更新
            vk::SemaphoreSubmitInfo::default()
                .semaphore(self.command_semaphore)
                .value(params.frame + 1)
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
        unsafe { device.queue_submit2(self.queue, &submit_infos, in_flight_fence) }.unwrap();

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

impl Drop for RenderingService {
    fn drop(&mut self) {
        let device = &self.device;
        unsafe { device.device_wait_idle().unwrap() }

        unsafe {
            device.destroy_sampler(self.glyph_sampler, None);
            device.destroy_image(self.glyph_image, None);
            device.destroy_image_view(self.glyph_image_view, None);
            device.free_memory(self.glyph_memory, None);
        }

        unsafe { device.destroy_descriptor_set_layout(self.descriptor_set_layout, None) };
        // unsafe { device.free_descriptor_sets(self.descriptor_pool, &self.descriptor_sets) }
        //     .unwrap();
        unsafe { device.destroy_descriptor_pool(self.descriptor_pool, None) };

        for pipeline in &self.pipelines {
            unsafe { device.destroy_pipeline(*pipeline, None) };
        }

        unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };

        unsafe { device.destroy_shader_module(self.shader_module, None) };

        for fence in &self.in_flight_fences {
            unsafe {
                device.destroy_fence(*fence, None);
            }
        }

        for semaphore in &self.command_completed_semaphores {
            unsafe { device.destroy_semaphore(*semaphore, None) };
        }
        unsafe {
            device.destroy_semaphore(self.command_semaphore, None);
        }

        for semaphore in &self.display_semaphores {
            unsafe { device.destroy_semaphore(*semaphore, None) };
        }

        unsafe { self.device.free_memory(self.copy_src_memory, None) };
        unsafe {
            self.device.destroy_buffer(self.copy_src_buffer, None);
        }
        unsafe { self.device.free_memory(self.buffer_memory, None) };
        unsafe { self.device.destroy_buffer(self.buffer, None) };

        unsafe {
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_utils_messanger, None)
        };

        unsafe { device.free_command_buffers(self.command_pool, &self.command_buffers) };
        unsafe { device.destroy_command_pool(self.command_pool, None) };

        while let Some(image_view) = self.present_image_views.pop() {
            unsafe { self.device.destroy_image_view(image_view, None) };
        }

        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };
        unsafe { self.surface_loader.destroy_surface(self.surface, None) };

        unsafe { self.device.destroy_device(None) };
        unsafe { self.instance.destroy_instance(None) };
    }
}

impl renge::ParametricService for RenderingService {
    type Params = RenderingServiceParams;

    async fn serve(self, params: Self::Params, cancellation_token: renge::CancellationToken) {
        self.serve(params, cancellation_token).await;
    }
}

impl CopyRange for vk::BufferCopy {
    fn new(src_offset: usize, dst_offset: usize, count: usize) -> Self {
        vk::BufferCopy::default()
            .src_offset(src_offset as vk::DeviceSize)
            .dst_offset(dst_offset as vk::DeviceSize)
            .size(count as vk::DeviceSize)
    }
}
