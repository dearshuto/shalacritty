use winit::window::WindowId;

use crate::{app::WindowSizeChangedEventArgs, Config};

use super::ImageLoadedEventArgs;

pub struct RenderingService<'a> {
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    image_receiver: tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,

    internal_instance: Instance<'a>,
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
        T: Into<wgpu::SurfaceTarget<'a>>,
    {
        Self {
            config_receiver,
            window_size_receiver,
            image_receiver,
            redraw_requested_window_id_receiver,
            internal_instance: Self::create_instance(window),
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(_config) = self.config_receiver.recv() => {},
            Some(args) = self.window_size_receiver.recv() => self.try_resize(args).await,
            Some(args) = self.image_receiver.recv() => self.apply_image(args),
            Some(window_id) = self.redraw_requested_window_id_receiver.recv() => self.redraw(window_id).await,
                                                        else => break,
                                                    );
        }
    }

    async fn try_resize(&mut self, args: WindowSizeChangedEventArgs) {
        let adapter = &self.internal_instance.adapter;
        let surface = &self.internal_instance.surface;

        let swapchain_format = surface.get_capabilities(adapter).formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: args.width,
            height: args.height,
            present_mode: wgpu::PresentMode::Fifo,
            #[cfg(not(any(target_os = "macos", windows)))]
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            #[cfg(target_os = "macos")]
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            #[cfg(target_os = "windows")]
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let device = &self.internal_instance.device;
        surface.configure(device, &config);
    }

    fn apply_image(&mut self, _args: ImageLoadedEventArgs) {
        let _instance = &self.internal_instance;

        // TODO
    }

    async fn redraw(&mut self, _id: WindowId) {
        let instance = &self.internal_instance;

        let device = &instance.device;
        let queue = &instance.queue;

        let Ok(frame) = instance.surface.get_current_texture() else {
            return;
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 背景
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&instance.background_pipeline);
            render_pass.set_bind_group(0, &instance.background_bind_group, &[]);
            render_pass.set_vertex_buffer(0, instance.rect_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                instance.rect_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        // 文字描画
        if 0 < instance.character_count {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&instance.text_pipeline);
            render_pass.set_bind_group(0, &instance.text_bind_gtoup, &[]);
            render_pass.set_vertex_buffer(0, instance.rect_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                instance.rect_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..6, 0, 0..instance.character_count);
        }
    }

    fn create_instance<T>(window: T) -> Instance<'a>
    where
        T: Into<wgpu::SurfaceTarget<'a>>,
    {
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window).unwrap();

        Instance {
            instance,
            surface,
            device: todo!(),
            queue: todo!(),
            adapter: todo!(),
            background_pipeline: todo!(),
            background_bind_group: todo!(),
            text_pipeline: todo!(),
            text_bind_gtoup: todo!(),
            character_count: todo!(),
            rect_vertex_buffer: todo!(),
            rect_index_buffer: todo!(),
        }
    }
}

struct Instance<'a> {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'a>,

    // 背景
    background_pipeline: wgpu::RenderPipeline,
    background_bind_group: wgpu::BindGroup,

    // テキスト描画
    text_pipeline: wgpu::RenderPipeline,
    text_bind_gtoup: wgpu::BindGroup,
    character_count: u32,

    rect_vertex_buffer: wgpu::Buffer,
    rect_index_buffer: wgpu::Buffer,
}
