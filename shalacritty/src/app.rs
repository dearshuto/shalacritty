use std::time::{Duration, Instant};

use tokio::sync::oneshot;
use winit::{
    event::{ElementState, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    keyboard::{ModifiersState, NamedKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

use crate::workspace::Workspace;

pub struct App {}

impl App {
    pub async fn run(is_profile_server_enabled: bool) {
        let (tx, rx) = oneshot::channel::<()>();

        let profiler_server_task = tokio::spawn(async move {
            if is_profile_server_enabled {
                profiler_core::Server::serve(([0, 0, 0, 0], 3030), rx).await;
            }
        });

        let mut modifiers_state = ModifiersState::empty();

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
                    WindowEvent::Ime(ime) => match ime {
                        winit::event::Ime::Enabled => {}
                        winit::event::Ime::Preedit(_, _) => {}
                        winit::event::Ime::Commit(str) => {
                            workspace.send_input(window_id, &str, modifiers_state)
                        }
                        winit::event::Ime::Disabled => {}
                    },
                    WindowEvent::Resized(size) => {
                        workspace.resize(window_id, size.width, size.height);
                    }
                    WindowEvent::RedrawRequested => {
                        workspace.render(window_id);
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        modifiers_state = modifiers.state();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state != ElementState::Pressed {
                            return;
                        }

                        if let Some(text) = event.text_with_all_modifiers() {
                            workspace.send_input(window_id, text, modifiers_state);
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
                            workspace.send_input(window_id, name_key, modifiers_state);
                        }

                        // 装飾キーが押されてると text_with_all_modifiers() が取得できないときがあるのでその救済措置
                        if let Some(text) = event.key_without_modifiers().to_text() {
                            workspace.send_input(window_id, text, modifiers_state);
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

        // サーバーば起動していたら終了要求を出す
        if is_profile_server_enabled {
            tx.send(()).unwrap();
        }
        profiler_server_task.await.unwrap();
    }
}
