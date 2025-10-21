use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use shaka::{
    BinarizeService, ConfigService, GlyphExtractService, InputAsynchronyzer, InputHandlingService,
    PatchService, RenderingService, ShellService, ShellServiceEvent,
};
use tokio::runtime::Runtime;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let event_loop = EventLoop::<ShellServiceEvent>::with_user_event()
        .build()
        .unwrap();

    let proxy = event_loop.create_proxy();

    event_loop
        .run_app(&mut App::new(proxy, Arc::new(runtime)))
        .unwrap();
}

struct App {
    runtime: Arc<Runtime>,
    event_loop_proxy: winit::event_loop::EventLoopProxy<ShellServiceEvent>,
    window: Option<Window>,

    service_runner: Option<renge::ServiceRunner>,
    input_sender: Option<std::sync::mpsc::Sender<KeyEvent>>,
}

impl App {
    pub fn new(
        event_loop_proxy: winit::event_loop::EventLoopProxy<ShellServiceEvent>,
        runtime: Arc<Runtime>,
    ) -> Self {
        Self {
            runtime,
            event_loop_proxy,
            window: None,
            service_runner: None,
            input_sender: None,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.input_sender = None;
        self.service_runner = None;
        self.window = None;
    }
}

//           resize ->
// config -> shell -> diff  -> binarize -> Render
//        -> Image    ↑
//        -> glyph  --
//
impl ApplicationHandler<ShellServiceEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_inner_size(PhysicalSize::new(640, 480))
            .with_visible(false);
        let window = event_loop.create_window(window_attributes).unwrap();

        let mut config_service = ConfigService::new();

        let (input_sender, input_receiver) = std::sync::mpsc::channel();
        let mut input_asynchronyzer = InputAsynchronyzer::new(input_receiver);

        let mut input_handling_service = InputHandlingService::new(input_asynchronyzer.listen());

        let (mut shell_service, diff_receiver) = ShellService::new(
            self.event_loop_proxy.clone(),
            input_handling_service.listen(),
        );

        let (_, rendering_service) = RenderingService::new(&window);

        let (glyph_service, glyph_receiver) =
            GlyphExtractService::new(config_service.listen(), shell_service.listen_str_diff());

        let (receiver, patch_service) = PatchService::new(glyph_receiver);

        let binarize_service = BinarizeService::new();

        let mut service_runner = renge::ServiceRunner::new(self.runtime.clone());
        service_runner.push(config_service);
        service_runner.push(input_asynchronyzer);
        service_runner.push(input_handling_service);
        service_runner.push(shell_service);
        service_runner.push(rendering_service);
        service_runner.push(glyph_service);
        service_runner.push(patch_service);
        service_runner.push(binarize_service);
        self.service_runner = Some(service_runner);

        self.window = Some(window);
        self.input_sender = Some(input_sender);
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
            WindowEvent::KeyboardInput {
                #[allow(unused)]
                device_id,
                event,
                #[allow(unused)]
                is_synthetic,
            } => {
                let Some(sender) = &self.input_sender else {
                    return;
                };

                sender.send(event).unwrap();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: ShellServiceEvent,
    ) {
        let _ = (event_loop, event);
    }
}
