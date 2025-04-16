use winit::window::WindowId;

use crate::{app::WindowSizeChangedEventArgs, Config};

use super::ImageLoadedEventArgs;

pub struct RenderingService<'a> {
    instance: wgpu::Instance,
    renderer: crate::gfx::Renderer<'a, ()>,
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    image_receiver: tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
    surface: Option<wgpu::Surface<'a>>,
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
        let renderer = crate::gfx::Renderer::new_with_plugin(());
        // 将来的にこっちに乗り換える
        let _surface = instance.create_surface(window).unwrap();

        Self {
            renderer,
            config_receiver,
            window_size_receiver,
            image_receiver,
            redraw_requested_window_id_receiver,
            instance,
            surface: None, /*Some(surface)*/
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(_config) = self.config_receiver.recv() => {},
            Some(args) = self.window_size_receiver.recv() => self.try_resize(args).await,
            Some(_args) = self.image_receiver.recv() => {},
            Some(window_id) = self.redraw_requested_window_id_receiver.recv() => self.redraw(window_id).await,
                                                    else => break,
                                                );
        }
    }

    async fn try_resize(&mut self, args: WindowSizeChangedEventArgs) {
        if let Some(surface) = self.surface.take() {
            self.renderer
                .register_with(args.id, &self.instance, surface)
                .await;
        }

        self.renderer.resize(args.id, args.width, args.height);
    }

    async fn redraw(&mut self, id: WindowId) {
        if let Some(surface) = self.surface.take() {
            self.renderer
                .register_with(id, &self.instance, surface)
                .await;
        }

        self.renderer.render(id);
    }
}
