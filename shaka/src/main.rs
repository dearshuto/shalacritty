use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use shaka::{BinarizeService, ConfigService, GlyphExtractService, RenderingService};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::builder().build().unwrap();
    event_loop.run_app(&mut App::new()).unwrap();
}

struct App {
    window: Option<Window>,
}

impl App {
    pub fn new() -> Self {
        Self { window: None }
    }
}

//           resize ->
// config -> shell -> diff  -> binarize -> Render
//        -> Image    |
//        -> glyph  <-            ↑
//             └ -> -> -> -> -> ->
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_inner_size(PhysicalSize::new(640, 480))
            .with_visible(false);
        let window = event_loop.create_window(window_attributes).unwrap();

        let config_service = ConfigService::new();

        let (_, rendering_service) = RenderingService::new(&window);
        tokio::spawn(rendering_service.serve());

        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let glyph_serrvice = GlyphExtractService::new(config_service.listen(), receiver);

        let binarize_service = BinarizeService::new(glyph_receiver);

        self.window = Some(window);
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_secs(1),
        ));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(window) = &self.window {
            if let Some(is_visible) = window.is_visible() {
                if !is_visible {
                    window.set_visible(true);
                }
            }
        }

        match event {
            WindowEvent::RedrawRequested => {}
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}
