mod buffer_view;
mod glyph_texture;
use glyph_texture::GlyphTexture;

use std::{borrow::Cow, io::Cursor, mem::offset_of};

use buffer_view::BufferView;

use ash::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::services::{
    glyph_extract_service::GlyphRequest, rendering_service::buffer_view::CharacterData,
};

pub struct RenderingServiceParams {
    pub receiver: tokio::sync::mpsc::Receiver<()>,
    pub glyph_request_sender: tokio::sync::mpsc::Sender<GlyphRequest>,
}

struct DrawParams {
    char_count: u32,
    frame: u64,
}

pub struct RenderingService {
    glyph_texture: GlyphTexture,

    instance: ash::Instance,
    device: ash::Device,
    #[allow(unused)]
    queue: vk::Queue,
    debug_utils_loader: ext::debug_utils::Instance,
    debug_utils_messanger: vk::DebugUtilsMessengerEXT,

    dynamic_rendering_device: khr::dynamic_rendering::Device,

    // Graphics Framework
    surface: vk::SurfaceKHR,
    surface_loader: khr::surface::Instance,
    swapchain_loader: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    #[allow(unused)]
    swapchain_images: Vec<vk::Image>,
    present_image_views: Vec<vk::ImageView>,

    // フレーム同期
    display_semaphores: Vec<vk::Semaphore>,
    data_copy_completed_semaphore: vk::Semaphore,
    command_completed_semaphores: Vec<vk::Semaphore>,
    command_fences: Vec<vk::Fence>,

    command_pool: vk::CommandPool,
    // 0, 1 →  描画コマンドのダブルバッファリング
    // 2 -> コピー用のコマンドバッファー
    command_buffers: Vec<vk::CommandBuffer>,

    // パイプライン関係
    shader_module: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    pipelines: Vec<vk::Pipeline>,

    // リソース
    buffer_memory: vk::DeviceMemory,
    copy_src_memory: vk::DeviceMemory,
    buffer: vk::Buffer,
    copy_src_buffer: vk::Buffer,
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
                // 文字の描画に必要な情報なのでいったんコメントアウト
                // vk::VertexInputAttributeDescription::default()
                //     .binding(1)
                //     .location(4)
                //     .format(vk::Format::R32G32B32A32_SFLOAT)
                //     .offset(offset_of!(CharacterData, uv01) as u32),
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

        const COPY_SRC_BUFFER_SIZE: vk::DeviceSize = 1024;
        let copy_src_buffer = {
            let queue_family_indices = [queue_family_index as u32];
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(&queue_family_indices)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .size(COPY_SRC_BUFFER_SIZE)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC);
            unsafe { device.create_buffer(&create_info, None) }.unwrap()
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
                16 * 1024,
                vk::MemoryMapFlags::empty(),
            )
        }
        .unwrap();

        let mut buffer_view = BufferView::new(ptr);
        {
            const VERTEX_DATA: [f32; 8] = [-0.5f32, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5];
            let vertex_buffer = buffer_view.vertex_buffer();
            vertex_buffer[0..VERTEX_DATA.len()].copy_from_slice(&VERTEX_DATA);
        }
        {
            const INDEX_DATA: [u16; 6] = [0, 1, 2, 0, 2, 3];
            let index_buffer = buffer_view.index_buffer();
            index_buffer[0..INDEX_DATA.len()].copy_from_slice(&INDEX_DATA);
        }

        // background_view.transform0 = [1.0, 0.0, 0.0, 1.0];
        // background_view.transform1 = [0.0, 1.0, 0.0, 1.0];

        unsafe {
            device.flush_mapped_memory_ranges(&[vk::MappedMemoryRange::default()
                .memory(device_memory)
                .offset(0)
                .size(1024)])
        }
        .unwrap();

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
                .command_buffer_count(3);
            unsafe { device.allocate_command_buffers(&allocate_info) }.unwrap()
        };

        let command_fences = {
            let create_info = vk::FenceCreateInfo::default();
            let fence0 = unsafe { device.create_fence(&create_info, None) }.unwrap();
            let fence1 = unsafe { device.create_fence(&create_info, None) }.unwrap();
            vec![fence0, fence1]
        };

        let (display_semaphores, command_completed_semaphores, data_copy_completed_semaphore) = {
            let create_info = vk::SemaphoreCreateInfo::default();
            let display_semaphore0 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            let display_semaphore1 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();

            let command_completed_semaphore0 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            let command_completed_semaphore1 =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();

            let data_copy_completed_semaphore =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            (
                vec![display_semaphore0, display_semaphore1],
                vec![command_completed_semaphore0, command_completed_semaphore1],
                data_copy_completed_semaphore,
            )
        };

        Self {
            glyph_texture: GlyphTexture::new(device.clone()),
            instance,
            device,
            debug_utils_loader,
            debug_utils_messanger: debug_utils,
            dynamic_rendering_device,
            display_semaphores,
            command_completed_semaphores,
            data_copy_completed_semaphore,
            command_fences,
            command_pool,
            command_buffers,
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
            buffer_memory: device_memory,
            copy_src_memory,
            buffer,
            copy_src_buffer,
        }
    }

    async fn serve(
        self,
        mut params: RenderingServiceParams,
        mut cancellation_token: renge::CancellationToken,
    ) {
        let mut receiver = params.receiver;

        // 本来は変更を受信したら呼び出す
        self.apply_patch("ABCDEFG", &mut params.glyph_request_sender)
            .await;

        let mut draw_params = DrawParams {
            char_count: 7,
            frame: 0,
        };
        loop {
            tokio::select! {
                Some(()) = receiver.recv() => {
                    self.draw(&draw_params);
                    draw_params.frame += 1;
                },
                _ = &mut cancellation_token => break,
                else => {},
            }
        }
    }

    async fn apply_patch(&self, str: &str, sender: &tokio::sync::mpsc::Sender<GlyphRequest>) {
        let (s, receiver) = tokio::sync::oneshot::channel();
        sender
            .send(GlyphRequest::new(&['A', 'B', 'C', 'D', 'E', 'F', 'G'], s).unwrap())
            .await
            .unwrap();

        let response = receiver.await.unwrap();
        self.glyph_texture.write(response.glyphs());

        let device = &self.device;

        unsafe {
            // GPU でコピー中だとデータ破壊が起きるので待つ
            // let semaphores = [self.data_copy_completed_semaphore];
            // let wait_info = vk::SemaphoreWaitInfo::default()
            //     .semaphores(&semaphores)
            //     .values(&[0]);
            // device.wait_semaphores(&wait_info, u64::MAX).unwrap();

            let ptr = device
                .map_memory(
                    self.copy_src_memory,
                    0,                                                           /*offset*/
                    std::mem::size_of::<CharacterData>() as vk::DeviceSize * 16, /*size*/
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap() as *mut CharacterData;
            let copy_src = std::slice::from_raw_parts_mut(ptr, 16);
            for index in 0..str.len() {
                let x = -0.9 + index as f32 * 0.3;
                copy_src[index].transform0 = [0.1, 0.0, x, 0.0];
                copy_src[index].transform1 = [0.0, 0.2, -0.5, 0.0];
                copy_src[index].fg_color = [0.0, 0.8, 0.0, 1.0];

                let range = self.glyph_texture.range('A');
                copy_src[index].uv01 = [
                    range.upper_right()[0],
                    range.upper_right()[1],
                    range.lower_left()[0],
                    range.lower_left()[1],
                ];
            }

            let ranges = [vk::MappedMemoryRange::default()
                .memory(self.copy_src_memory)
                .offset(0)
                .size((std::mem::size_of::<CharacterData>() * str.len()) as vk::DeviceSize)];
            device.flush_mapped_memory_ranges(&ranges).unwrap();
            device.unmap_memory(self.copy_src_memory);
        };

        let command_buffer = self.command_buffers[2];
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default();
            device
                .begin_command_buffer(command_buffer, &begin_info)
                .unwrap();

            let regions = [vk::BufferCopy::default()
                .src_offset(0)
                .dst_offset(BufferView::character_data_offset())
                .size((std::mem::size_of::<CharacterData>() * str.len()) as vk::DeviceSize)];
            device.cmd_copy_buffer(command_buffer, self.copy_src_buffer, self.buffer, &regions);

            device.end_command_buffer(command_buffer).unwrap();
        };
    }

    fn draw(&self, params: &DrawParams) {
        let device = &self.device;
        let display_semaphore = self.display_semaphores[0];
        let command_completed_semaphore = self.command_completed_semaphores[0];
        let next_command_fence = self.command_fences[0];
        let command_buffer = self.command_buffers[0];
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

            // ひとつのバッファーを分割してふたつの頂点データとして利用
            // 矩形をシェーダー上で生成してしまえば頂点分のデータはいらなくなるかも
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0, /*first_binding*/
                &[self.buffer, self.buffer],
                &[
                    BufferView::vertex_buffer_offset(),
                    BufferView::character_data_offset(),
                ],
            );

            device.cmd_bind_index_buffer(
                command_buffer,
                self.buffer,
                (std::mem::size_of::<f32>() * 16) as u64, /*offset*/
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

        // データ更新のパッチ適用コマンド
        let is_data_copy_required = true;
        if is_data_copy_required {
            // コピーだけなので StageMask はなくてよい？
            let wait_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [self.command_buffers[2]];
            // TODO: 本当は前回の描画コマンドの終了を待つ
            let wait_semaphores = [display_semaphore];
            let signal_semaphores = [self.data_copy_completed_semaphore];
            let submit_info = [vk::SubmitInfo::default()
                .wait_dst_stage_mask(&wait_mask)
                .command_buffers(&command_buffers)
                .wait_semaphores(&wait_semaphores)
                .signal_semaphores(&signal_semaphores)];
            unsafe { device.queue_submit(self.queue, &submit_info, vk::Fence::null()) }.unwrap();
        }

        {
            let wait_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [command_buffer];
            // データコピーのコマンドを待つ
            let wait_semaphores = [self.data_copy_completed_semaphore];
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

        for pipeline in &self.pipelines {
            unsafe { device.destroy_pipeline(*pipeline, None) };
        }

        unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };

        unsafe { device.destroy_shader_module(self.shader_module, None) };

        for semaphore in &self.command_completed_semaphores {
            unsafe { device.destroy_semaphore(*semaphore, None) };
        }

        unsafe {
            device.destroy_semaphore(self.data_copy_completed_semaphore, None);
        }
        for semaphore in &self.display_semaphores {
            unsafe { device.destroy_semaphore(*semaphore, None) };
        }

        for fence in &self.command_fences {
            unsafe { device.destroy_fence(*fence, None) };
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
