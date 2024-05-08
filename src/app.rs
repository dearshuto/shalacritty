use std::time::{Duration, Instant};

use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    keyboard::ModifiersState,
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

use crate::workspace::{Input, Modifier, Workspace};

pub struct App;

impl App {
    pub async fn run() {
        let mut modifier = Modifier::empty();
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
                        if !event.state.is_pressed() {
                            return;
                        }

                        let Some(text) = event.text_with_all_modifiers() else {
                            return;
                        };

                        let input = Input {
                            modifiers: modifier,
                            text,
                        };

                        workspace.send_input(window_id, &input);
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        if modifiers.state().contains(ModifiersState::CONTROL) {
                            modifier = Modifier::CONTROL;
                        } else {
                            modifier = Modifier::empty();
                        }
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
