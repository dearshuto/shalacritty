use std::time::{Duration, Instant};

use winit::{
    event::{ElementState, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::workspace::Workspace;

pub struct App;

impl App {
    pub async fn run() {
        let event_loop = EventLoopBuilder::new().build().unwrap();

        // ひとつだけウィンドウを起動しておく
        let mut workspace = Workspace::new();
        workspace.spawn_window(&event_loop).await;

        let timer_length = Duration::from_millis(10);
        event_loop
            .run(move |event, target| match event {
                Event::NewEvents(StartCause::Init) => {
                    target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + timer_length))
                }
                Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                    target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + timer_length));
                    workspace.update();
                }
                Event::WindowEvent {
                    window_id, event, ..
                } => match event {
                    WindowEvent::Resized(size) => {
                        workspace.resize(window_id, size.width, size.width);
                    }
                    WindowEvent::RedrawRequested => {
                        workspace.render(window_id);
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state != ElementState::Pressed {
                            return;
                        }

                        let text = crate::convert_key_to_str(event);
                        workspace.send(window_id, &text);
                    }
                    WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    _ => {}
                },
                _ => {}
            })
            .unwrap();
    }
}
