use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use shaka::{
    BinarizeService, ConfigService, GlyphExtractService, PatchService, RenderingService,
    ShellService,
};
use tokio::runtime::Runtime;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let event_loop = EventLoop::builder().build().unwrap();
    event_loop
        .run_app(&mut App::new(Arc::new(runtime)))
        .unwrap();
}

struct App {
    runtime: Arc<Runtime>,
    window: Option<Window>,

    service_runner: Option<renge::ServiceRunner>,
}

impl App {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            window: None,
            service_runner: None,
        }
    }
}

//           resize ->
// config -> shell -> diff  -> binarize -> Render
//        -> Image    ↑
//        -> glyph  --
//
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_inner_size(PhysicalSize::new(640, 480))
            .with_visible(false);
        let window = event_loop.create_window(window_attributes).unwrap();

        let mut config_service = ConfigService::new();
        let mut shell_service = ShellService::new();

        let (_, rendering_service) = RenderingService::new(&window);

        let (glyph_serrvice, glyph_receiver) =
            GlyphExtractService::new(config_service.listen(), shell_service.listen_str_diff());

        // let (receiver, patch_service) = PatchService::new(glyph_receiver);

        let binarize_service = BinarizeService::new();

        let mut service_runner = renge::ServiceRunner::new(self.runtime.clone());
        service_runner.push(rendering_service);
        // service_runner.push(patch_service);
        service_runner.push(binarize_service);
        self.service_runner = Some(service_runner);

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
