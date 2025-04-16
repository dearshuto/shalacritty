use winit::window::WindowId;

use crate::{app::WindowSizeChangedEventArgs, Config};

use super::ImageLoadedEventArgs;

pub struct RenderingService<'a> {
    instance: wgpu::Instance,
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    image_receiver: tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
    surface: Option<wgpu::Surface<'a>>,

    internal_instance: Option<Instance<'a>>,
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
        let instance = wgpu::Instance::default();
        // 将来的にこっちに乗り換える
        let _surface = instance.create_surface(window).unwrap();

        Self {
            config_receiver,
            window_size_receiver,
            image_receiver,
            redraw_requested_window_id_receiver,
            instance,
            surface: None, /*Some(surface)*/
            internal_instance: None,
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

    async fn try_resize(&mut self, args: WindowSizeChangedEventArgs) {}

    fn apply_image(&mut self, _args: ImageLoadedEventArgs) {
        let Some(_instance) = &self.internal_instance else {
            return;
        };

        // TODO
    }

    async fn redraw(&mut self, _id: WindowId) {
        let Some(instance) = &self.internal_instance else {
            return;
        };

        let device = &instance.device;
        let queue = &instance.queue;

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 文字描画
        {
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
        }
    }
}

struct Instance<'a> {
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
