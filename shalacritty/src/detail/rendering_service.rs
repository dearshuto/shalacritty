use std::sync::Arc;

use crate::{
    app::{RedrawRequestedEventArgs, WindowCreatedEventArgs, WindowSizeChangedEventArgs},
    gfx::{CharacterInfo, RendererUpdateParams},
    Config,
};

pub struct RenderingService<'a> {
    renderer: crate::gfx::Renderer<'a, ()>,
    instance: wgpu::Instance,

    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_created_receiver: tokio::sync::mpsc::Receiver<WindowCreatedEventArgs>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<RedrawRequestedEventArgs>,

    window: Option<Arc<winit::window::Window>>,
}

impl<'a> RenderingService<'a> {
    pub fn new(
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        window_created_receiver: tokio::sync::mpsc::Receiver<WindowCreatedEventArgs>,
        window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<RedrawRequestedEventArgs>,
    ) -> Self {
        Self {
            renderer: crate::gfx::Renderer::new_with_plugin(()),
            instance: wgpu::Instance::default(),
            config_receiver,
            window_created_receiver,
            window_size_receiver,
            redraw_requested_window_id_receiver,
            window: None,
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(_config) = self.config_receiver.recv() => {},
            Some(args) = self.window_created_receiver.recv() => self.crated(args).await,
            Some(args) = self.window_size_receiver.recv() => self.try_resize(args).await,
            Some(args) = self.redraw_requested_window_id_receiver.recv() => self.redraw(args),
                                                else => break,
                                            );
        }
    }

    async fn crated(&mut self, args: WindowCreatedEventArgs) {
        self.window = Some(args.window);
    }

    async fn try_resize(&mut self, args: WindowSizeChangedEventArgs) {
        let mut window = None;
        std::mem::swap(&mut window, &mut self.window);

        if window.is_some() {
            self.renderer
                .register(args.id, &self.instance, window.unwrap())
                .await;
        }

        self.renderer.resize(args.id, args.width, args.height);
    }

    fn redraw(&mut self, args: RedrawRequestedEventArgs) {
        if !self.window.is_none() {
            return;
        }

        let diff = crate::gfx::Diff {
            character_info_array: vec![CharacterInfo {
                code: 'a',
                transform: nalgebra::Matrix3x2::identity(),
                fore_ground_color: [0.0f32; 4],
                uv0: nalgebra::Vector2::new(0.0, 0.0),
                uv1: nalgebra::Vector2::new(0.0, 0.0),
                index: 0,
            }],
            cursor: None,
            item_count: 1,
        };
        self.renderer.update_with_user_data(
            args.id,
            &RendererUpdateParams::<String, ()>::new_with_user_data(())
                .with_diff(diff)
                .with_background_color(Some([0.1, 0.2, 0.3, 1.0])),
        );

        self.renderer.render(args.id);
    }
}
