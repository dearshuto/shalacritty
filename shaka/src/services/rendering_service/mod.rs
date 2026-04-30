mod buffer_layout;
mod buffer_view;
mod glyph_table;
mod range_allocator;
mod renderer;
mod text_writer;
mod transfer_queue;

use std::{borrow::Cow, u64};

use ash::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::services::{
    EventKind,
    glyph_extract_service::GlyphRequest,
    rendering_service::{
        renderer::Renderer, text_writer::CopyRange, transfer_queue::TransferQueue,
    },
    shell_service::{PatchData, TextData},
};

pub struct RenderingServiceParams {
    pub receiver: tokio::sync::mpsc::Receiver<()>,
    pub content_receiver: tokio::sync::mpsc::Receiver<TextData>,
    pub glyph_request_sender: tokio::sync::mpsc::Sender<GlyphRequest>,
    #[allow(unused)]
    pub resize_receiver: tokio::sync::broadcast::Receiver<EventKind>,
}

pub struct RenderingService {
    entry: ash::Entry,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
}

struct DrawParams {
    char_count: u32,
    frame: u64,
    image_layout: vk::ImageLayout,
    transfer_queue: TransferQueue<PatchData>,
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

        Self {
            entry,
            instance,
            surface,
        }
    }

    async fn serve(
        self,
        mut params: RenderingServiceParams,
        mut cancellation_token: renge::CancellationToken,
    ) {
        let mut receiver = params.receiver;
        let mut content_receiver = params.content_receiver;

        let mut renderer = Renderer::new(&self.entry, &self.instance, self.surface);

        let mut draw_params = DrawParams {
            char_count: 64,
            frame: 0,
            image_layout: vk::ImageLayout::UNDEFINED,
            transfer_queue: TransferQueue::new(),
        };
        let mut patch_buffer = [Default::default(); 64];
        loop {
            tokio::select! {
                Some(()) = receiver.recv() => {
                    renderer.draw(&draw_params);
                    draw_params.frame += 1;
                    draw_params.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                },
                Some(text_data) = content_receiver.recv() => {
                    draw_params.char_count = text_data.char_count as u32;
                    draw_params.transfer_queue.push(&mut patch_buffer, &text_data.patches);
                    // 内部で draw を呼び出します
                    renderer.apply_patch(&self.instance, &draw_params, &patch_buffer, &mut params.glyph_request_sender)
                        .await;
                    draw_params.frame += 1;
                    draw_params.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
                },
                _ = &mut cancellation_token => break,
                else => {},
            }
        }
    }
}

impl Drop for RenderingService {
    fn drop(&mut self) {
        //     let device = &self.device;
        //     unsafe { device.device_wait_idle().unwrap() }

        //     unsafe {
        //         device.destroy_sampler(self.glyph_sampler, None);
        //         device.destroy_image(self.glyph_image, None);
        //         device.destroy_image_view(self.glyph_image_view, None);
        //         device.free_memory(self.glyph_memory, None);
        //     }

        //     unsafe { device.destroy_descriptor_set_layout(self.descriptor_set_layout, None) };
        //     // unsafe { device.free_descriptor_sets(self.descriptor_pool, &self.descriptor_sets) }
        //     //     .unwrap();
        //     unsafe { device.destroy_descriptor_pool(self.descriptor_pool, None) };

        //     for pipeline in &self.pipelines {
        //         unsafe { device.destroy_pipeline(*pipeline, None) };
        //     }

        //     unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };

        //     unsafe { device.destroy_shader_module(self.shader_module, None) };

        //     for fence in &self.in_flight_fences {
        //         unsafe {
        //             device.destroy_fence(*fence, None);
        //         }
        //     }

        //     for semaphore in &self.command_completed_semaphores {
        //         unsafe { device.destroy_semaphore(*semaphore, None) };
        //     }
        //     unsafe {
        //         device.destroy_semaphore(self.command_semaphore, None);
        //     }

        //     for semaphore in &self.display_semaphores {
        //         unsafe { device.destroy_semaphore(*semaphore, None) };
        //     }

        //     unsafe { self.device.free_memory(self.copy_src_memory, None) };
        //     unsafe {
        //         self.device.destroy_buffer(self.copy_src_buffer, None);
        //     }
        //     unsafe { self.device.free_memory(self.buffer_memory, None) };
        //     unsafe { self.device.destroy_buffer(self.buffer, None) };

        //     unsafe {
        //         self.debug_utils_loader
        //             .destroy_debug_utils_messenger(self.debug_utils_messanger, None)
        //     };

        //     unsafe { device.free_command_buffers(self.command_pool, &self.command_buffers) };
        //     unsafe { device.destroy_command_pool(self.command_pool, None) };

        //     while let Some(image_view) = self.present_image_views.pop() {
        //         unsafe { self.device.destroy_image_view(image_view, None) };
        //     }

        //     unsafe {
        //         self.swapchain_loader
        //             .destroy_swapchain(self.swapchain, None)
        //     };
        //     unsafe { self.surface_loader.destroy_surface(self.surface, None) };

        //     unsafe { self.device.destroy_device(None) };
        //     unsafe { self.instance.destroy_instance(None) };
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
