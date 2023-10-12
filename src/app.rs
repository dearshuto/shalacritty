use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

#[allow(unused_imports)]
use crate::window::WindowManager;

pub struct App;

impl App {
    pub async fn run() {
        let event_loop = EventLoopBuilder::new().build();

        let mut window_manager = WindowManager::new();
        window_manager.create_window(&event_loop);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }
        });
    }
}
