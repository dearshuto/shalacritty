use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use profiler_core::IServerBackend;
use tokio::sync::oneshot;
use winit::{
    event::{ElementState, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    keyboard::{ModifiersState, NamedKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

use crate::workspace::{IWorkspaceCallback, Workspace};

pub struct App {}

impl App {
    pub fn run(is_profile_server_enabled: bool) {
        let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();

        let server_backend = ServerBackend::new();
        let server_backend_local = server_backend.clone();

        let (tx, rx) = oneshot::channel::<()>();
        let profiler_server_task = runtime.spawn(async move {
            if is_profile_server_enabled {
                profiler_core::Server::serve(([0, 0, 0, 0], 3030), server_backend_local, rx).await;
            }
        });

        let mut modifiers_state = ModifiersState::empty();

        let event_loop = EventLoopBuilder::new().build().unwrap();

        // ひとつだけウィンドウを起動しておく
        let mut workspace = runtime.block_on(async {
            let mut workspace = Workspace::new_with_callback(server_backend);
            workspace.spawn_window(&event_loop).await;

            workspace
        });

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

        runtime.block_on(async {
            profiler_server_task.await.unwrap();
        });
    }
}

#[derive(Clone)]
struct ServerBackendImpl {
    begin_table: HashMap<String, std::time::SystemTime>,
    duration: HashMap<String, VecDeque<std::time::Duration>>,
}

#[derive(Clone)]
struct ServerBackend {
    server_backend_impl: Arc<Mutex<ServerBackendImpl>>,
}

impl ServerBackend {
    pub fn new() -> Self {
        Self {
            server_backend_impl: Arc::new(Mutex::new(ServerBackendImpl {
                begin_table: Default::default(),
                duration: Default::default(),
            })),
        }
    }
}

impl IServerBackend for ServerBackend {
    fn count(&self) -> usize {
        self.server_backend_impl.lock().unwrap().duration.len()
    }

    fn key(&self, index: usize) -> String {
        let binding = self.server_backend_impl.lock().unwrap();
        let key = binding.duration.keys().nth(index).unwrap();
        key.to_string()
    }

    fn cache_count(&self, key: &str) -> usize {
        let binding = self.server_backend_impl.lock().unwrap();
        binding.duration[key].len()
    }

    fn duration(&self, key: &str, index: usize) -> std::time::Duration {
        let binding = self.server_backend_impl.lock().unwrap();
        let queue = binding.duration.get(key).unwrap();
        queue[index]
    }
}

impl IWorkspaceCallback for ServerBackend {
    fn begin(&mut self, time: std::time::SystemTime, id: &str) {
        self.server_backend_impl
            .lock()
            .unwrap()
            .begin_table
            .insert(id.to_string(), time);
    }

    fn end(&mut self, time: std::time::SystemTime, id: &str) {
        let mut binding = self.server_backend_impl.lock().unwrap();
        let Some(begin) = binding.begin_table.get(id) else {
            return;
        };

        let duration = time.duration_since(*begin).unwrap();
        let Some(queue) = binding.duration.get_mut(id) else {
            // 初回挿入
            binding
                .duration
                .insert(id.to_string(), VecDeque::from([duration]));
            return;
        };

        // 新しいデータは末尾に挿入
        queue.push_back(duration);

        // キャッシュするのは最大 100 個まで
        if 100 <= queue.len() {
            queue.pop_front();
        }
    }
}
