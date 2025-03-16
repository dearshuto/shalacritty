use winit::window::WindowId;

use crate::{
    app::{WindowCreatedEventArgs, WindowSizeChangedEventArgs},
    Config,
};

pub struct RenderingService<'a> {
    renderer: crate::gfx::Renderer<'a, ()>,
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_created_receiver: tokio::sync::mpsc::Receiver<WindowCreatedEventArgs>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
}

impl<'a> RenderingService<'a> {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        window_created_receiver: tokio::sync::mpsc::Receiver<WindowCreatedEventArgs>,
        window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
    ) -> Self {
        Self {
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
            Some(args) = self.window_size_receiver.recv() => self.try_resize(args),
            Some(window_id) = self.redraw_requested_window_id_receiver.recv() => self.redraw(window_id),
                                                else => break,
                                            );
        }
    }

    async fn crated(&mut self, _args: WindowCreatedEventArgs) {
        // TODO: Window の寿命を適切に管理する実装を考える
    }

    fn try_resize(&mut self, args: WindowSizeChangedEventArgs) {
        self.renderer.resize(args.id, args.width, args.height);
    }

    fn redraw(&mut self, id: WindowId) {
        self.renderer.render(id);
    }
}
