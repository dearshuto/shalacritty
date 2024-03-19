use std::time::{Duration, Instant};

use winit::{
    event::{ElementState, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    keyboard::NamedKey,
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

                        if let Some(text) = event.text_with_all_modifiers() {
                            workspace.send(window_id, text);
                            return;
                        };

                        if let Some(name_key) = match event.logical_key {
                            winit::keyboard::Key::Named(key) => match key {
                                NamedKey::ArrowUp => Some("ArrowUp"),
                                NamedKey::ArrowDown => Some("ArrowDown"),
                                NamedKey::ArrowRight => Some("ArrowRight"),
                                NamedKey::ArrowLeft => Some("ArrowLeft"),
                                _ => None,
                            },
                            // winit::keyboard::Key::Character(_) => {}
                            // winit::keyboard::Key::Unidentified(_) => {}
                            // winit::keyboard::Key::Dead(_) => {}
                            _ => None,
                        } {
                            workspace.send(window_id, name_key);
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
