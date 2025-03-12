use raw_window_handle::{DisplayHandle, HasDisplayHandle, HasWindowHandle, WindowHandle};
use winit::window::WindowId;

use crate::{
    app::{WindowCreatedEventArgs, WindowSizeChangedEventArgs},
    Config,
};

pub struct RenderingService<'a> {
    instance: wgpu::Instance,
    renderer: crate::gfx::Renderer<'a, ()>,
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_created_receiver: tokio::sync::mpsc::Receiver<WindowCreatedEventArgs<'a>>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
}

impl<'a> RenderingService<'a> {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        window_created_receiver: tokio::sync::mpsc::Receiver<WindowCreatedEventArgs<'a>>,
        window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
    ) -> Self {
        Self {
            instance: wgpu::Instance::default(),
            renderer: crate::gfx::Renderer::new_with_plugin(()),
            config_receiver,
            window_created_receiver,
            window_size_receiver,
            redraw_requested_window_id_receiver,
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
                    Some(_config) = self.config_receiver.recv() => {},
            Some(args) = self.window_created_receiver.recv() => self.crated(args).await,
                    Some(args) = self.window_size_receiver.recv() => self.resize(args),
                    Some(window_id) = self.redraw_requested_window_id_receiver.recv() => self.redraw(window_id),
                                            else => break,
                                        );
        }
    }

    async fn crated(&mut self, args: WindowCreatedEventArgs<'a>) {
        self.renderer
            .register(
                args.id,
                &self.instance,
                WindowAdapter {
                    raw_window_handle: args.raw_window_handle,
                    raw_display_handle: args.raw_display_handle,
                },
            )
            .await;
    }

    fn resize(&mut self, args: WindowSizeChangedEventArgs) {
        self.renderer.resize(args.id, args.width, args.height);
    }

    fn redraw(&mut self, id: WindowId) {
        self.renderer.render(id);
    }
}

struct WindowAdapter<'a> {
    raw_window_handle: WindowHandle<'a>,
    raw_display_handle: DisplayHandle<'a>,
}

impl<'a> HasWindowHandle for WindowAdapter<'a> {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        Ok(self.raw_window_handle)
    }
}

impl<'a> HasDisplayHandle for WindowAdapter<'a> {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(self.raw_display_handle)
    }
}
