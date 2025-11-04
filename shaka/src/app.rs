use renge::ServiceRunner;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoopProxy,
    window::{Window, WindowAttributes},
};

use crate::services::RenderingService;

pub struct UserEvent {}

pub struct App {
    window: Option<Window>,
    service_runner: renge::DefaultServiceRunner,
    #[allow(unused)]
    event_loop_proxy: EventLoopProxy<UserEvent>,
    redraw_request_sender: Option<tokio::sync::mpsc::Sender<()>>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            window: None,
            service_runner: ServiceRunner::default(),
            event_loop_proxy: proxy,
            redraw_request_sender: None,
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        let window = event_loop.create_window(window_attributes).unwrap();

        let rendering_service = RenderingService::new(&window);

        let (redraw_request_sender, redraw_request_receiver) = tokio::sync::mpsc::channel(1);
        self.service_runner
            .push_with_params(rendering_service, redraw_request_receiver);

        self.window = Some(window);
        self.redraw_request_sender = Some(redraw_request_sender);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, _event: UserEvent) {
        let Some(window) = &self.window else {
            return;
        };

        window.request_redraw();
    }
}
