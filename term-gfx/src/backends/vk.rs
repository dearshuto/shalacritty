use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::{c_char, c_void},
    u64,
};

use ash::ext::debug_utils;
use ash::*;
use raw_window_handle::{DisplayHandle, WindowHandle};
use util::Align;

use crate::{
    detail,
    traits::{IMapHandle, RenderParams, UpdateDescriptorsParams},
    IBackend,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RenderTargetId {
    internal: uuid::Uuid,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SemaphoreId {
    internal: uuid::Uuid,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct BufferId {
    internal: uuid::Uuid,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PipelineId {
    internal: uuid::Uuid,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct DescriptorSetId {
    internal: uuid::Uuid,
}

pub struct BackendVk {
    entry: ash::Entry,
    instance_table: HashMap<RenderTargetId, ash::Instance>,
    debug_utils_instance_table: HashMap<RenderTargetId, debug_utils::Instance>,
    debug_util_messanger_table: HashMap<RenderTargetId, ash::vk::DebugUtilsMessengerEXT>,
    device_table: HashMap<RenderTargetId, ash::Device>,
    physical_device_table: HashMap<RenderTargetId, ash::vk::PhysicalDevice>,
    queue_table: HashMap<RenderTargetId, ash::vk::Queue>,
    surface_instance_table: HashMap<RenderTargetId, ash::khr::surface::Instance>,
    surface_table: HashMap<RenderTargetId, ash::vk::SurfaceKHR>,
    command_pool: HashMap<RenderTargetId, ash::vk::CommandPool>,
    command_buffer_table: HashMap<RenderTargetId, [ash::vk::CommandBuffer; 2]>,
    swapchain_table: HashMap<RenderTargetId, ash::vk::SwapchainKHR>,
    swapchain_loader_table: HashMap<RenderTargetId, ash::khr::swapchain::Device>,
    swapchain_image_view_table: HashMap<RenderTargetId, [ash::vk::ImageView; 2]>,
    frame_buffers_table: HashMap<RenderTargetId, [ash::vk::Framebuffer; 2]>,
    fence_table: HashMap<RenderTargetId, ash::vk::Fence>,

    //  セマフォ
    semaphore_table: HashMap<RenderTargetId, HashMap<SemaphoreId, ash::vk::Semaphore>>,

    // デバイスメモリーとバッファーは 1:1 対応しているので同じ id を割り振っておく
    buffer_table: HashMap<RenderTargetId, HashMap<BufferId, ash::vk::Buffer>>,
    device_memory_table: HashMap<RenderTargetId, HashMap<BufferId, ash::vk::DeviceMemory>>,

    // サンプラー
    sampler_table: HashMap<RenderTargetId, ash::vk::Sampler>,

    // レンダーパス
    render_pass_table: HashMap<RenderTargetId, ash::vk::RenderPass>,

    // 描画パイプライン
    pipeline_table: HashMap<RenderTargetId, HashMap<PipelineId, ash::vk::Pipeline>>,

    // デスクリプタープール
    descriptor_pool_table: HashMap<RenderTargetId, ash::vk::DescriptorPool>,

    // デスクリプターセット
    descriptor_set_table: HashMap<RenderTargetId, HashMap<DescriptorSetId, ash::vk::DescriptorSet>>,

    // デスクリプターセットレイアウト
    descriptor_set_layout_table:
        HashMap<RenderTargetId, HashMap<PipelineId, ash::vk::DescriptorSetLayout>>,

    // パイプラインレイアウト
    pipeline_layout_table: HashMap<RenderTargetId, Vec<ash::vk::PipelineLayout>>,

    // シェーダー
    shader_table: HashMap<RenderTargetId, Vec<ash::vk::ShaderModule>>,
}

impl BackendVk {
    fn find_memorytype_index(
        memory_req: &ash::vk::MemoryRequirements,
        memory_prop: &ash::vk::PhysicalDeviceMemoryProperties,
        flags: ash::vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        memory_prop.memory_types[..memory_prop.memory_type_count as _]
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                (1 << index) & memory_req.memory_type_bits != 0
                    && memory_type.property_flags & flags == flags
            })
            .map(|(index, _memory_type)| index as _)
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

impl IBackend for BackendVk {
    type RenderTargetId = RenderTargetId;
    type SemaphoreId = SemaphoreId;
    type PipelineId = PipelineId;
    type DescriptorSetId = DescriptorSetId;
    type BufferId = BufferId;
    type MapHandle = MapHandle;

    fn new() -> BackendVk {
        let entry = ash::Entry::linked();

        Self {
            entry,
            instance_table: HashMap::default(),
            debug_utils_instance_table: HashMap::default(),
            debug_util_messanger_table: HashMap::default(),
            device_table: HashMap::default(),
            physical_device_table: HashMap::default(),
            queue_table: HashMap::default(),
            surface_instance_table: HashMap::default(),
            surface_table: HashMap::default(),
            frame_buffers_table: HashMap::default(),
            swapchain_table: HashMap::default(),
            swapchain_loader_table: HashMap::default(),
            swapchain_image_view_table: HashMap::default(),
            command_pool: HashMap::default(),
            command_buffer_table: HashMap::default(),
            fence_table: HashMap::default(),
            semaphore_table: HashMap::default(),
            buffer_table: HashMap::default(),
            device_memory_table: HashMap::default(),
            render_pass_table: HashMap::default(),
            pipeline_table: HashMap::default(),
            pipeline_layout_table: HashMap::default(),
            shader_table: HashMap::default(),
            descriptor_pool_table: HashMap::default(),
            descriptor_set_table: HashMap::default(),
            descriptor_set_layout_table: HashMap::default(),
            sampler_table: HashMap::default(),
        }
    }

    fn register_surface(
        &mut self,
        window_handle: WindowHandle,
        display_handle: DisplayHandle,
    ) -> Result<Self::RenderTargetId, ()> {
        let instance = unsafe {
            let appinfo = ash::vk::ApplicationInfo::default()
                .application_name(c"MyName")
                .engine_name(c"MyName")
                .application_version(0)
                .engine_version(0)
                .api_version(ash::vk::make_api_version(0, 1, 0, 0));
            let mut extension_names =
                ash_window::enumerate_required_extensions(display_handle.as_raw())
                    .unwrap()
                    .to_vec();
            extension_names.append(&mut vec![
                ash::ext::debug_utils::NAME.as_ptr(),
                ash::khr::get_physical_device_properties2::NAME.as_ptr(),
                ash::khr::portability_enumeration::NAME.as_ptr(),
            ]);
            extension_names.push(debug_utils::NAME.as_ptr());

            let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                ash::vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                ash::vk::InstanceCreateFlags::default()
            };

            let layer_names = [c"VK_LAYER_KHRONOS_validation"];
            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();
            let instance_create_info = ash::vk::InstanceCreateInfo::default()
                .application_info(&appinfo)
                .enabled_layer_names(&layers_names_raw)
                .enabled_extension_names(&extension_names)
                .flags(create_flags);
            self.entry.create_instance(&instance_create_info, None)
        }
        .unwrap();

        let debug_utils_loader = debug_utils::Instance::new(&self.entry, &instance);

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
        let raw_window_handle = window_handle.as_raw();
        let raw_dispay_handle = display_handle.as_raw();
        let surface = unsafe {
            ash_window::create_surface(
                &self.entry,
                &instance,
                raw_dispay_handle,
                raw_window_handle,
                None,
            )
        }
        .unwrap();

        // 物理デバイスの検索
        let surface_loader = ash::khr::surface::Instance::new(&self.entry, &instance);
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
        let present_image_view: Vec<_> = swapchain_images
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

        // フェンス
        let fence = {
            let fence_create_info =
                ash::vk::FenceCreateInfo::default().flags(ash::vk::FenceCreateFlags::SIGNALED);
            unsafe { device.create_fence(&fence_create_info, None) }.unwrap()
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

        // インスタンスの保持
        let id = RenderTargetId {
            internal: uuid::Uuid::new_v4(),
        };
        self.instance_table.insert(id, instance);
        self.debug_utils_instance_table
            .insert(id, debug_utils_loader);
        self.debug_util_messanger_table.insert(id, debug_utils);
        self.swapchain_table.insert(id, swapchain);
        self.swapchain_loader_table.insert(id, swapchain_loader);
        self.surface_instance_table.insert(id, surface_loader);
        self.surface_table.insert(id, surface);
        self.device_table.insert(id, device);
        self.physical_device_table.insert(id, physical_device);
        self.queue_table.insert(id, queue);
        self.command_pool.insert(id, command_pool);
        self.command_buffer_table
            .insert(id, [command_buffers[0], command_buffers[1]]);
        self.swapchain_image_view_table
            .insert(id, [present_image_view[0], present_image_view[1]]);
        self.fence_table.insert(id, fence);
        self.descriptor_pool_table.insert(id, descriptor_pool);
        self.sampler_table.insert(id, sampler);

        Ok(id)
    }

    fn create_semaphore(&mut self, id: Self::RenderTargetId) -> Result<Self::SemaphoreId, ()> {
        let Some(device) = self.device_table.get(&id) else {
            return Err(());
        };

        // セマフォ
        let semaphore_create_info = ash::vk::SemaphoreCreateInfo::default();
        let semaphore = unsafe { device.create_semaphore(&semaphore_create_info, None) }.unwrap();

        let semaphore_id = SemaphoreId {
            internal: uuid::Uuid::now_v7(),
        };
        self.semaphore_table
            .entry(id)
            .and_modify(|x| {
                x.insert(semaphore_id, semaphore);
            })
            .or_insert(HashMap::from([(semaphore_id, semaphore)]));
        Ok(semaphore_id)
    }

    fn create_pipeline(
        &mut self,
        id: Self::RenderTargetId,
        vertex_shader_spv: &[u8],
        pixel_shader_spv: &[u8],
    ) -> Result<Self::PipelineId, ()> {
        let Some(device) = self.device_table.get(&id) else {
            return Err(());
        };

        let Some(present_image_views) = self.swapchain_image_view_table.get(&id) else {
            return Err(());
        };

        let shader_reflection = detail::ShaderReflection::new();
        let vertex_stage_layout = shader_reflection.parse(vertex_shader_spv).unwrap();

        let vertex_shader_spv = unsafe {
            &*std::ptr::slice_from_raw_parts(
                vertex_shader_spv.as_ptr() as *const u32,
                vertex_shader_spv.len() / 4,
            )
        };
        let pixel_shader_spv = unsafe {
            &*std::ptr::slice_from_raw_parts(
                pixel_shader_spv.as_ptr() as *const u32,
                pixel_shader_spv.len() / 4,
            )
        };
        let vertex_shader_module_create_info =
            vk::ShaderModuleCreateInfo::default().code(vertex_shader_spv);
        let pixel_shader_module_create_info =
            vk::ShaderModuleCreateInfo::default().code(pixel_shader_spv);

        let vertex_shader_module =
            unsafe { device.create_shader_module(&vertex_shader_module_create_info, None) }
                .unwrap();
        let pixel_shader_module =
            unsafe { device.create_shader_module(&pixel_shader_module_create_info, None) }.unwrap();

        let descriptor_set_layout_bindings: Vec<_> = vertex_stage_layout
            .resources()
            .iter()
            .map(|resource| match resource {
                detail::ResourceType::Uniform(binding) => {
                    ash::vk::DescriptorSetLayoutBinding::default()
                        .binding(*binding)
                        .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
                }
                detail::ResourceType::UnorderedAccessBuffer(binding) => {
                    ash::vk::DescriptorSetLayoutBinding::default()
                        .binding(*binding)
                        .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(ash::vk::ShaderStageFlags::VERTEX)
                }
                _ => todo!(),
            })
            .collect();
        let descriptor_set_create_info = ash::vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&descriptor_set_layout_bindings);
        let descriptor_set_layouts =
            [
                unsafe { device.create_descriptor_set_layout(&descriptor_set_create_info, None) }
                    .unwrap(),
            ];

        let pipeline_layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_set_layouts);
        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None) }.unwrap();

        let shader_entry_name = c"main";
        let shader_stage_create_infos = [
            ash::vk::PipelineShaderStageCreateInfo {
                module: vertex_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: ash::vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            ash::vk::PipelineShaderStageCreateInfo {
                s_type: ash::vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                module: pixel_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: ash::vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];

        let vertex_attribute_descriptions = [ash::vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(ash::vk::Format::R32G32_SFLOAT)];
        let vertex_binding_descriptions = [ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<f32>() as u32 * 2)];
        let pipeline_vertex_input_state_create_info =
            ash::vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_attribute_descriptions(&vertex_attribute_descriptions)
                .vertex_binding_descriptions(&vertex_binding_descriptions);
        let input_assembly_state = ash::vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(ash::vk::PrimitiveTopology::TRIANGLE_LIST);

        let dynamic_states = [
            ash::vk::DynamicState::VIEWPORT,
            ash::vk::DynamicState::SCISSOR,
        ];
        let dynamic_state =
            ash::vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let renderpass_attachments = [ash::vk::AttachmentDescription {
            format: ash::vk::Format::B8G8R8A8_UNORM,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
            final_layout: ash::vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        }];
        let color_attachments = [ash::vk::AttachmentReference {
            attachment: 0,
            layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let subpass_description = [
            // 背景
            ash::vk::SubpassDescription::default()
                .color_attachments(&color_attachments)
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS),
            // 文字
            ash::vk::SubpassDescription::default()
                .color_attachments(&color_attachments)
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS),
            // カーソル
            ash::vk::SubpassDescription::default()
                .color_attachments(&color_attachments)
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS),
            // 矩形(拡張予定)
            ash::vk::SubpassDescription::default()
                .color_attachments(&color_attachments)
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS),
        ];
        let render_pass = unsafe {
            device.create_render_pass(
                &ash::vk::RenderPassCreateInfo::default()
                    .attachments(&renderpass_attachments)
                    .subpasses(&subpass_description),
                None,
            )
        }
        .unwrap();
        let frame_buffers: Vec<_> = present_image_views
            .iter()
            .map(|view| {
                let framebuffer_attachments = [*view];
                let frame_buffer_create_info = ash::vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&framebuffer_attachments)
                    .width(640)
                    .height(480)
                    .layers(1);

                unsafe { device.create_framebuffer(&frame_buffer_create_info, None) }.unwrap()
            })
            .collect();
        let viewports = [ash::vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 480.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: ash::vk::Extent2D {
                width: 640,
                height: 480,
            },
        }];
        let viewport_state_info = ash::vk::PipelineViewportStateCreateInfo::default()
            .scissors(&scissors)
            .viewports(&viewports);
        let rasterization_info = ash::vk::PipelineRasterizationStateCreateInfo {
            front_face: ash::vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            polygon_mode: ash::vk::PolygonMode::FILL,
            ..Default::default()
        };
        let color_blend_attachment_states = [ash::vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(ash::vk::ColorComponentFlags::RGBA)];
        let color_blend_state = ash::vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);
        let multisample_state_info = ash::vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: ash::vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let graphics_pipeline_create_info = ash::vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&pipeline_vertex_input_state_create_info)
            .dynamic_state(&dynamic_state)
            .multisample_state(&multisample_state_info)
            .viewport_state(&&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .color_blend_state(&&color_blend_state)
            .render_pass(render_pass)
            .input_assembly_state(&input_assembly_state)
            .layout(pipeline_layout);

        let pipeline = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[graphics_pipeline_create_info],
                None,
            )
        }
        .unwrap();

        let pipeline_id = PipelineId {
            internal: uuid::Uuid::new_v4(),
        };
        self.pipeline_table
            .entry(id)
            .and_modify(|id| {
                id.insert(pipeline_id, pipeline[0]);
            })
            .or_insert(HashMap::from([(pipeline_id, pipeline[0])]));
        self.pipeline_layout_table
            .entry(id)
            .and_modify(|vec| vec.push(pipeline_layout))
            .or_insert(vec![pipeline_layout]);
        self.shader_table
            .entry(id)
            .and_modify(|vec| {
                vec.push(vertex_shader_module);
                vec.push(pixel_shader_module)
            })
            .or_insert(vec![vertex_shader_module, pixel_shader_module]);
        self.render_pass_table.insert(id, render_pass);
        self.frame_buffers_table
            .insert(id, [frame_buffers[0], frame_buffers[1]]);

        self.descriptor_set_layout_table
            .entry(id)
            .and_modify(|table| {
                table.insert(pipeline_id, descriptor_set_layouts[0]);
            })
            .or_insert(HashMap::from([(pipeline_id, descriptor_set_layouts[0])]));

        Ok(pipeline_id)
    }

    fn allocate_descriptor_set(
        &mut self,
        id: Self::RenderTargetId,
        pipeline_id: Self::PipelineId,
    ) -> Result<Self::DescriptorSetId, ()> {
        let Some(device) = self.device_table.get(&id) else {
            return Err(());
        };

        let Some(descriptor_pool) = self.descriptor_pool_table.get(&id) else {
            return Err(());
        };

        let Some(descriptor_set_layout_table) = self.descriptor_set_layout_table.get(&id) else {
            return Err(());
        };

        let Some(descriptor_set_layout) = descriptor_set_layout_table.get(&pipeline_id) else {
            return Err(());
        };

        let descriptor_set_layouts = [*descriptor_set_layout];
        let descriptor_set_allocate_info = ash::vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(*descriptor_pool)
            .set_layouts(&descriptor_set_layouts);
        let descriptor_sets =
            unsafe { device.allocate_descriptor_sets(&descriptor_set_allocate_info) }.unwrap();

        let descriptor_set_id = DescriptorSetId {
            internal: uuid::Uuid::now_v7(),
        };
        self.descriptor_set_table
            .entry(id)
            .and_modify(|x| {
                x.insert(descriptor_set_id, descriptor_sets[0]);
            })
            .or_insert(HashMap::from([(descriptor_set_id, descriptor_sets[0])]));

        Ok(descriptor_set_id)
    }

    fn update_descriptors(
        &mut self,
        update_descriptors_params: &UpdateDescriptorsParams<
            Self::RenderTargetId,
            Self::DescriptorSetId,
            Self::BufferId,
        >,
    ) {
        let render_target_id = update_descriptors_params.render_target_id;
        let descriptor_set_id = update_descriptors_params.descriptor_set_id;

        let Some(device) = self.device_table.get(&render_target_id) else {
            return;
        };

        let Some(descriptor_set_table) = self.descriptor_set_table.get(&render_target_id) else {
            return;
        };

        let Some(descriptor_set) = descriptor_set_table.get(&descriptor_set_id) else {
            return;
        };

        let buffer_info: Vec<_> = update_descriptors_params
            .buffer_info
            .iter()
            .filter_map(|info| {
                let Some(buffer_table) = self.buffer_table.get(&render_target_id) else {
                    return None;
                };

                let Some(buffer) = buffer_table.get(&info.id) else {
                    return None;
                };

                Some(
                    ash::vk::DescriptorBufferInfo::default()
                        .offset(info.offset as ash::vk::DeviceSize)
                        .range(info.size as ash::vk::DeviceSize)
                        .buffer(*buffer),
                )
            })
            .collect();

        let write_descriptor_sets = [ash::vk::WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_set(*descriptor_set)
            .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_info)];
        unsafe { device.update_descriptor_sets(&write_descriptor_sets, &[]) };
    }

    fn allocate_buffer(
        &mut self,
        id: Self::RenderTargetId,
        size: usize,
        usage: crate::traits::BufferUsage,
    ) -> Result<Self::BufferId, ()> {
        let Some(device) = self.device_table.get(&id) else {
            return Err(());
        };

        let Some(instance) = self.instance_table.get(&id) else {
            return Err(());
        };

        let Some(physical_device) = self.physical_device_table.get(&id) else {
            return Err(());
        };

        let buffer_usafe_flag = match usage {
            crate::traits::BufferUsage::IndexBuffer => ash::vk::BufferUsageFlags::INDEX_BUFFER,
            crate::traits::BufferUsage::VertexBuffer => ash::vk::BufferUsageFlags::VERTEX_BUFFER,
            crate::traits::BufferUsage::UniformBuffer => ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
            crate::traits::BufferUsage::UnorderedAccessBuffer => {
                ash::vk::BufferUsageFlags::STORAGE_BUFFER
            }
            crate::traits::BufferUsage::CopySource => ash::vk::BufferUsageFlags::TRANSFER_SRC,
        };
        let buffer_create_info = ash::vk::BufferCreateInfo::default()
            .usage(buffer_usafe_flag)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .size(size as u64);
        let buffer = unsafe { device.create_buffer(&buffer_create_info, None) }.unwrap();

        let device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(*physical_device) };
        let buffer_requirement = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type_index = Self::find_memorytype_index(
            &buffer_requirement,
            &device_memory_properties,
            ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
        )
        .unwrap();
        let memory_allocate_info = ash::vk::MemoryAllocateInfo::default()
            .allocation_size(size as u64)
            .memory_type_index(memory_type_index);

        let device_memory = unsafe { device.allocate_memory(&memory_allocate_info, None) }.unwrap();

        // デバイスメモリーとバッファーを紐づける
        // デバイスメモリーとバッファーは 1:1 対応なので、デバイスメモリー全体をバッファーに紐づける
        unsafe {
            device.bind_buffer_memory(buffer, device_memory, 0 /*offset*/)
        }
        .unwrap();

        let buffer_id = BufferId {
            internal: uuid::Uuid::new_v4(),
        };
        self.buffer_table
            .entry(id)
            .and_modify(|x| {
                x.insert(buffer_id, buffer);
            })
            .or_insert(HashMap::from([(buffer_id, buffer)]));
        self.device_memory_table
            .entry(id)
            .and_modify(|x| {
                x.insert(buffer_id, device_memory);
            })
            .or_insert(HashMap::from([(buffer_id, device_memory)]));

        Ok(buffer_id)
    }

    fn destroy_buffer(
        &mut self,
        #[allow(unused)] id: Self::RenderTargetId,
        #[allow(unused)] buffer_id: Self::BufferId,
    ) {
        // TODO: Renderer から破棄を要求できるようにしたい
        // 現状は Drop 時にまとめて破棄しているのでリークの心配はない
    }

    fn map_buffer(
        &mut self,
        id: Self::RenderTargetId,
        buffer_id: Self::BufferId,
    ) -> Result<Self::MapHandle, ()> {
        let Some(device) = self.device_table.get(&id) else {
            return Err(());
        };

        let Some(device_memory_table) = self.device_memory_table.get(&id) else {
            return Err(());
        };
        let Some(device_memory) = device_memory_table.get(&buffer_id) else {
            return Err(());
        };

        let index_ptr =
            unsafe { device.map_memory(*device_memory, 0, 64, ash::vk::MemoryMapFlags::empty()) }
                .unwrap();

        Ok(MapHandle {
            device: device.clone(),
            device_memory: *device_memory,
            index_ptr,
        })
    }

    fn flush_buffer(
        &mut self,
        id: Self::RenderTargetId,
        buffer_id: Self::BufferId,
        offset: usize,
        size: usize,
    ) {
        let Some(device) = self.device_table.get(&id) else {
            return;
        };

        let Some(device_memory_table) = self.device_memory_table.get(&id) else {
            return;
        };

        let Some(device_memory) = device_memory_table.get(&buffer_id) else {
            return;
        };

        let range = ash::vk::MappedMemoryRange::default()
            .memory(*device_memory)
            .offset(offset as ash::vk::DeviceSize)
            .size(size as u64);
        unsafe { device.flush_mapped_memory_ranges(&[range]) }.unwrap();
    }

    fn acquire_next_frame(
        &mut self,
        id: Self::RenderTargetId,
        semaphore_id: Self::SemaphoreId,
    ) -> Result<u32, ()> {
        let Some(device) = self.device_table.get(&id) else {
            return Err(());
        };

        let Some(queue) = self.queue_table.get(&id) else {
            return Err(());
        };

        let Some(swapchain_loader) = self.swapchain_loader_table.get(&id) else {
            return Err(());
        };

        let Some(swapchain) = self.swapchain_table.get(&id) else {
            return Err(());
        };

        let Some(semaphore_table) = self.semaphore_table.get(&id) else {
            return Err(());
        };

        let Some(semaphore) = semaphore_table.get(&semaphore_id) else {
            return Err(());
        };

        unsafe { device.queue_wait_idle(*queue) }.unwrap();

        // フレームバッファを要求
        let (present_index, _) = unsafe {
            swapchain_loader.acquire_next_image(
                *swapchain,
                u64::MAX,
                *semaphore,
                ash::vk::Fence::null(),
            )
        }
        .unwrap();

        Ok(present_index)
    }

    fn render(
        &self,
        render_params: RenderParams<
            Self::RenderTargetId,
            Self::SemaphoreId,
            Self::PipelineId,
            Self::DescriptorSetId,
            Self::BufferId,
        >,
    ) {
        let target_id = render_params.render_target_id;

        // デバイス
        let Some(device) = self.device_table.get(&target_id) else {
            return;
        };

        // キュー
        let Some(queue) = self.queue_table.get(&target_id) else {
            return;
        };

        // コマンドバッファー
        let Some(command_buffers) = self.command_buffer_table.get(&target_id) else {
            return;
        };
        let command_buffer = command_buffers[0];

        // フェンス
        let Some(fence) = self.fence_table.get(&target_id) else {
            return;
        };

        // セマフォ
        let Some(semaphore_table) = self.semaphore_table.get(&target_id) else {
            return;
        };
        let Some(acquire_next_frame_semaphore) =
            semaphore_table.get(&render_params.acquire_next_frame_semaphore_id)
        else {
            return;
        };
        let Some(queue_submit_semaphore) =
            semaphore_table.get(&render_params.queue_submit_signal_semaphore_id)
        else {
            return;
        };

        // レンダーパス
        let Some(render_pass) = self.render_pass_table.get(&render_params.render_target_id) else {
            return;
        };

        // フレームバッファー
        let Some(frame_buffers) = self
            .frame_buffers_table
            .get(&render_params.render_target_id)
        else {
            return;
        };

        // パイプライン
        let Some(pipeline_table) = self.pipeline_table.get(&target_id) else {
            return;
        };
        let Some(pipeline) = pipeline_table.get(&render_params.pipelie_id) else {
            return;
        };

        // パイプラインレイアウト
        let Some(pipeline_layouts) = self.pipeline_layout_table.get(&target_id) else {
            return;
        };

        // バッファー一覧
        let Some(buffer_table) = self.buffer_table.get(&target_id) else {
            return;
        };

        // 頂点バッファー
        let Some(vertex_buffer) = buffer_table.get(&render_params.vertex_buffer_id) else {
            return;
        };

        // インデックスバッファー
        let Some(index_buffer) = buffer_table.get(&render_params.index_buffer_id) else {
            return;
        };

        // コマンドの完了まち
        unsafe {
            device.wait_for_fences(&[*fence], true, u64::MAX /*timeout*/)
        }
        .unwrap();

        // フェンスのシグナルをクリア
        unsafe { device.reset_fences(&[*fence]) }.unwrap();

        // コマンド作成開始
        let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo::default()
            .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info) }.unwrap();

        // レンダーパス
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.1, 0.2, 0.3, 0.0],
            },
        }];
        let render_pass_begin_info = ash::vk::RenderPassBeginInfo::default()
            .render_pass(*render_pass)
            .framebuffer(frame_buffers[render_params.process_index as usize])
            .render_area(ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: 0, y: 0 },
                extent: ash::vk::Extent2D {
                    width: 640,
                    height: 480,
                },
            })
            .clear_values(&clear_values);
        unsafe {
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                ash::vk::SubpassContents::INLINE,
            )
        };

        // パイプライン
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                ash::vk::PipelineBindPoint::GRAPHICS,
                *pipeline,
            )
        }

        // ビューポートシザー
        unsafe {
            device.cmd_set_viewport(
                command_buffer,
                0,
                &[ash::vk::Viewport::default()
                    .x(0.0)
                    .y(0.0)
                    .width(640.0)
                    .height(480.0)],
            );

            device.cmd_set_scissor(
                command_buffer,
                0,
                &[ash::vk::Rect2D::default()
                    .extent(ash::vk::Extent2D::default().width(640).height(480))],
            );
        }

        // 頂点バッファ
        unsafe {
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0, /*first_binding*/
                &[*vertex_buffer],
                &[0], /*offsets*/
            )
        }

        // インデックスバッファー
        unsafe {
            device.cmd_bind_index_buffer(
                command_buffer,
                *index_buffer,
                0, /*offset*/
                ash::vk::IndexType::UINT32,
            );
        }

        // リソースたち
        // MEMO: ネストが深いのは改善できる？
        if let Some(descriptor_set_id) = render_params.descriptor_set_id {
            if let Some(descriptor_set_table) = self
                .descriptor_set_table
                .get(&render_params.render_target_id)
            {
                if let Some(descriptor_set) = descriptor_set_table.get(&descriptor_set_id) {
                    unsafe {
                        device.cmd_bind_descriptor_sets(
                            command_buffer,
                            ash::vk::PipelineBindPoint::GRAPHICS,
                            pipeline_layouts[0],
                            0,
                            &[*descriptor_set],
                            &[],
                        )
                    }
                }
            }
        }

        // インスタンス描画
        // 一部パラメーターは固定
        unsafe {
            device.cmd_draw_indexed(
                command_buffer,
                render_params.index_count,
                render_params.instance_count,
                0, /*first_index*/
                0, /*vertex_offset*/
                0, /*first_instance*/
            );
        }

        // 未実装のサブパス分は未使用のまま見送る
        unsafe { device.cmd_next_subpass(command_buffer, ash::vk::SubpassContents::INLINE) }
        unsafe { device.cmd_next_subpass(command_buffer, ash::vk::SubpassContents::INLINE) }
        unsafe { device.cmd_next_subpass(command_buffer, ash::vk::SubpassContents::INLINE) }

        // レンダーパス終わり
        unsafe { device.cmd_end_render_pass(command_buffer) }

        unsafe { device.end_command_buffer(command_buffer) }.unwrap();

        // コマンドの提出
        let signal_semaphores = [*queue_submit_semaphore];
        let wait_semaphores = [*acquire_next_frame_semaphore];
        let command_buffers = [command_buffer];
        let submit_info = ash::vk::SubmitInfo::default()
            .command_buffers(&command_buffers)
            .wait_dst_stage_mask(&[ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .wait_semaphores(&wait_semaphores)
            .signal_semaphores(&signal_semaphores);
        unsafe { device.queue_submit(*queue, &[submit_info], *fence) }.unwrap();
    }

    fn present(
        &self,
        process_index: u32,
        id: Self::RenderTargetId,
        wait_semaphore_id: Self::SemaphoreId,
    ) {
        // セマフォ
        let Some(semaphore_table) = self.semaphore_table.get(&id) else {
            return;
        };
        let Some(semaphore) = semaphore_table.get(&wait_semaphore_id) else {
            return;
        };

        // スワップチェーン
        let Some(swapchain) = self.swapchain_table.get(&id) else {
            return;
        };
        let Some(swapchain_loader) = self.swapchain_loader_table.get(&id) else {
            return;
        };

        // キュー
        let Some(queue) = self.queue_table.get(&id) else {
            return;
        };

        // 画面に表示
        let wait_semaphors = [*semaphore];
        let swapchains = [*swapchain];
        // TODO: ダブルバッファー対応
        let image_indices = [process_index];
        let present_info = ash::vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphors)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        unsafe { swapchain_loader.queue_present(*queue, &present_info) }.unwrap();
    }
}

impl Drop for BackendVk {
    fn drop(&mut self) {
        for (render_target_id, device) in &self.device_table {
            // キューの完了待ち
            if let Some(queue) = self.queue_table.remove(render_target_id) {
                unsafe { device.queue_wait_idle(queue) }.unwrap()
            }

            // デスクリプター関係
            if let Some(descriptor_set_layouts) =
                self.descriptor_set_layout_table.remove(render_target_id)
            {
                for (_, descriptor_set_layout) in descriptor_set_layouts {
                    unsafe { device.destroy_descriptor_set_layout(descriptor_set_layout, None) }
                }
            }
            if let Some(descriptor_pool) = self.descriptor_pool_table.get(render_target_id) {
                for table in self.descriptor_set_table.values() {
                    for descriptor_set in table.values() {
                        unsafe {
                            device.free_descriptor_sets(*descriptor_pool, &[*descriptor_set])
                        }
                        .unwrap();
                    }
                }

                unsafe { device.destroy_descriptor_pool(*descriptor_pool, None) };
            }

            // フェンス
            if let Some(fence) = self.fence_table.remove(render_target_id) {
                unsafe { device.destroy_fence(fence, None) }
            }

            // コマンドプール
            if let Some(command_pool) = self.command_pool.remove(render_target_id) {
                // コマンドバッファー
                if let Some(command_buffers) = self.command_buffer_table.remove(render_target_id) {
                    unsafe { device.free_command_buffers(command_pool, &command_buffers) }
                }

                unsafe { device.destroy_command_pool(command_pool, None) }
            }

            // サンプラー
            for sampler in self.sampler_table.values() {
                unsafe { device.destroy_sampler(*sampler, None) }
            }

            // デバイスメモリー
            if let Some(device_memory_table) = self.device_memory_table.remove(render_target_id) {
                for device_memory in device_memory_table.values() {
                    unsafe { device.free_memory(*device_memory, None) }
                }
            }

            // バッファー
            if let Some(buffer_table) = self.buffer_table.remove(render_target_id) {
                for buffer in buffer_table.values() {
                    unsafe { device.destroy_buffer(*buffer, None) }
                }
            }

            // レンダーパス
            if let Some(render_pass) = self.render_pass_table.remove(render_target_id) {
                unsafe { device.destroy_render_pass(render_pass, None) }
            }

            // シェーダー
            if let Some(shader_modules) = self.shader_table.get(render_target_id) {
                for shader_module in shader_modules {
                    unsafe { device.destroy_shader_module(*shader_module, None) }
                }
            }

            // パイプラインレイアウト
            if let Some(pipeline_layouts) = self.pipeline_layout_table.remove(render_target_id) {
                for pipeline_layout in pipeline_layouts {
                    unsafe { device.destroy_pipeline_layout(pipeline_layout, None) }
                }
            }

            // パイプライン
            if let Some(pipeline_table) = self.pipeline_table.remove(render_target_id) {
                for pipeline in pipeline_table.values() {
                    unsafe { device.destroy_pipeline(*pipeline, None) }
                }
            }

            // セマフォ
            if let Some(semaphore_table) = self.semaphore_table.remove(render_target_id) {
                for semaphore in semaphore_table.values() {
                    unsafe { device.destroy_semaphore(*semaphore, None) }
                }
            }

            // フレームバッファー
            if let Some(frame_buffers) = self.frame_buffers_table.remove(render_target_id) {
                unsafe { device.destroy_framebuffer(frame_buffers[0], None) }
                unsafe { device.destroy_framebuffer(frame_buffers[1], None) }
            }

            // スワップチェーンのイメージ
            if let Some(image_views) = self.swapchain_image_view_table.remove(render_target_id) {
                unsafe { device.destroy_image_view(image_views[0], None) }
                unsafe { device.destroy_image_view(image_views[1], None) }
            }

            // スワップチェーン
            if let Some(swapchain_loader) = self.swapchain_loader_table.remove(render_target_id) {
                if let Some(swapchain) = self.swapchain_table.remove(render_target_id) {
                    unsafe { swapchain_loader.destroy_swapchain(swapchain, None) }
                }
            }

            // サーフェース
            // スワップチェーンよりも後に破棄しなければならない
            if let Some(surface_loader) = self.surface_instance_table.remove(render_target_id) {
                if let Some(surface) = self.surface_table.remove(render_target_id) {
                    unsafe { surface_loader.destroy_surface(surface, None) }
                }
            }

            // デバッグ情報
            if let Some(instance) = self.debug_utils_instance_table.get(&render_target_id) {
                if let Some(debug_utils_messanger) =
                    self.debug_util_messanger_table.remove(render_target_id)
                {
                    unsafe { instance.destroy_debug_utils_messenger(debug_utils_messanger, None) }
                }
            }

            // デバイス
            unsafe { device.destroy_device(None) };

            // インスタンス
            if let Some(instance) = self.instance_table.get(render_target_id) {
                unsafe { instance.destroy_instance(None) }
            }
        }
    }
}

pub struct MapHandle {
    device: ash::Device,
    device_memory: ash::vk::DeviceMemory,
    index_ptr: *mut c_void,
}

impl IMapHandle for MapHandle {
    fn write(&mut self, _offset: usize, data: &[u8]) {
        let mut index_slice = unsafe {
            Align::new(
                self.index_ptr,
                align_of::<u8>() as u64,
                data.len() as ash::vk::DeviceSize,
            )
        };
        index_slice.copy_from_slice(data);
    }
}

impl Drop for MapHandle {
    fn drop(&mut self) {
        unsafe { self.device.unmap_memory(self.device_memory) }
    }
}
