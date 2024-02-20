use std::time::{Duration, Instant};

use winit::{
    event::{ElementState, Event, KeyEvent, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
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

                    if workspace.is_empty() {
                        target.exit();
                    }
                }
                Event::WindowEvent {
                    window_id, event, ..
                } => match event {
                    WindowEvent::Resized(size) => {
                        workspace.resize(window_id, size.width, size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        workspace.render(window_id);
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state != ElementState::Pressed {
                            return;
                        }

                        let text = Self::convert_key_to_str(event);
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

    fn convert_key_to_str(key_event: KeyEvent) -> String {
        let Some(text) = key_event.text_with_all_modifiers() else {
            return "".to_string();
        };

        text.to_string()
    }
}
