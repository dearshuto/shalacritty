use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::{tty::TeletypeManager, window::WindowManager};

pub struct App;

impl App {
    pub async fn run() {
        let event_loop = EventLoopBuilder::new().build();

        let mut window_manager = WindowManager::new();
        window_manager.create_window(&event_loop);

        let mut teletype_manager = TeletypeManager::new();
        let _tty_id = teletype_manager.create_teletype();

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
