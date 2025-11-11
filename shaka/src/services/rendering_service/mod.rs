mod buffer_view;
use std::{borrow::Cow, io::Cursor};

use buffer_view::BufferView;

use ash::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::services::{GraphicsServiceProxy, graphics_service::ShaderModuleRequest};

pub struct RenderingService {
    graphics_service_proxy: GraphicsServiceProxy,

    // パイプライン関係
    shader_module: vk::ShaderModule,
    pipeline_layout: vk::PipelineLayout,
    pipelines: Vec<vk::Pipeline>,

    // リソース
    buffer_memory: vk::DeviceMemory,
    buffer: vk::Buffer,
}

impl RenderingService {
    pub fn new(proxy: GraphicsServiceProxy) -> Self {
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
        let character_data = buffer_view.character_data();

        // background_view.transform0 = [1.0, 0.0, 0.0, 1.0];
        // background_view.transform1 = [0.0, 1.0, 0.0, 1.0];
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
            debug_utils_loader,
            debug_utils_messanger: debug_utils,
            dynamic_rendering_device,
            display_semaphores,
            command_completed_semaphores,
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
            buffer,
        }
    }

    async fn serve(
        self,
        mut receiver: tokio::sync::mpsc::Receiver<()>,
        mut cancellation_token: renge::CancellationToken,
    ) {
        let shader_module = self
            .graphics_service_proxy
            .request_shader_module(ShaderModuleRequest {})
            .await;

        let mut frame = 0;
        loop {
            tokio::select! {
                Some(()) = receiver.recv() => self.draw(&mut frame),
                _ = &mut cancellation_token => break,
                else => {},
            }
        }
    }

    fn draw(&self, frame: &mut u64) {
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

            device.cmd_bind_vertex_buffers(
                command_buffer,
                0, /*first_binding*/
                &[self.buffer],
                &[0],
            );

            device.cmd_bind_index_buffer(
                command_buffer,
                self.buffer,
                (std::mem::size_of::<f32>() * 16) as u64, /*offset*/
                vk::IndexType::UINT16,
            );

            device.cmd_draw_indexed(
                command_buffer,
                6, /*index_count*/
                1, /*instance_count*/
                0, /*first_index*/
                0, /*vertex_offset*/
                0, /*first_instance*/
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

        *frame += 1;
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

        for semaphore in &self.display_semaphores {
            unsafe { device.destroy_semaphore(*semaphore, None) };
        }

        for fence in &self.command_fences {
            unsafe { device.destroy_fence(*fence, None) };
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
    type Params = tokio::sync::mpsc::Receiver<()>;

    async fn serve(self, params: Self::Params, cancellation_token: renge::CancellationToken) {
        self.serve(params, cancellation_token).await;
    }
}
