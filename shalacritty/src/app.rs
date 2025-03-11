use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use profiler_core::IServerBackend;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tokio::{sync::oneshot, task::JoinHandle};

use winit::{
    application::ApplicationHandler,
    event::{ElementState, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{ModifiersState, NamedKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

use crate::workspace::{IWorkspaceCallback, Workspace};

pub struct App<'a, TBackend>
where
    TBackend: term_gfx::IBackend,
{
    window_table: HashMap<winit::window::WindowId, winit::window::Window>,
    workspace: Workspace<'a, ServerBackend>,
    renderer: term_gfx::Renderer<TBackend>,
    modifiers_state: ModifiersState,
    profiler_server_task: JoinHandle<()>,
    profiler_kill_sender: oneshot::Sender<()>,

    // シェル管理（載せ替え予定）
    multiplexer: asura::Multiplexer,

    runtime: Arc<tokio::runtime::Runtime>,
}

impl<'a, TBackend> App<'a, TBackend>
where
    TBackend: term_gfx::IBackend,
{
    pub fn new(renderer: term_gfx::Renderer<TBackend>, is_profile_server_enabled: bool) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_time()
                .build()
                .unwrap(),
        );

        let server_backend = ServerBackend::new();
        let server_backend_local = server_backend.clone();

        let (tx, rx) = oneshot::channel::<()>();
        let profiler_server_task = runtime.spawn(async move {
            if is_profile_server_enabled {
                profiler_core::Server::serve(([0, 0, 0, 0], 3030), server_backend_local, rx).await;
            }
        });

        let workspace = Workspace::new_with_callback(runtime.clone(), server_backend);

        Self {
            runtime,
            window_table: HashMap::default(),
            workspace,
            renderer,
            modifiers_state: ModifiersState::default(),
            profiler_server_task,
            profiler_kill_sender: tx,
            multiplexer: asura::Multiplexer::new(),
        }
    }

    pub fn run(renderer: term_gfx::Renderer<TBackend>, is_profile_server_enabled: bool) {
        let event_loop = EventLoop::builder().build().unwrap();

        let mut app = Self::new(renderer, is_profile_server_enabled);
        event_loop.run_app(&mut app).unwrap();

        if is_profile_server_enabled {
            app.profiler_kill_sender.send(()).unwrap();
            app.runtime.block_on(async {
                app.profiler_server_task.await.unwrap();
            });
        }
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

impl<'a, TBackend> ApplicationHandler for App<'a, TBackend>
where
    TBackend: term_gfx::IBackend,
{
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // ひとつだけウィンドウを起動しておく
        let window_attributes = winit::window::WindowAttributes::default()
            .with_transparent(true)
            .with_min_inner_size(winit::dpi::PhysicalSize::new(300, 300))
            .with_max_inner_size(winit::dpi::PhysicalSize::new(4096, 4096));
        let window = event_loop.create_window(window_attributes).unwrap();
        let (width, height) = (window.inner_size().width, window.inner_size().height);
        let id = window.id();
        window.set_ime_allowed(true);

        self.runtime.block_on(async {
            self.workspace
                .assign_window(id, &window, width, height)
                .await;
        });

        // ひとつのウィンドウハンドルにはひとつの swapchain しか作成できないのでいったん無効化
        if false {
            let window_handle = window.window_handle().unwrap();
            let display_handle = window.display_handle().unwrap();
            self.renderer
                .register_surface(window_handle, display_handle)
                .unwrap();
        }

        self.window_table.insert(id, window);

        let timer_length = Duration::from_millis(10);
        let control_flow = ControlFlow::WaitUntil(Instant::now() + timer_length);
        event_loop.set_control_flow(control_flow);
    }

    fn new_events(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, cause: StartCause) {
        match cause {
            StartCause::ResumeTimeReached { .. } => {
                let Some((id, window)) = self.window_table.iter().next() else {
                    return;
                };

                self.workspace
                    .update(*id, window.inner_size().width, window.inner_size().height);
                if self.workspace.is_empty() {
                    event_loop.exit();
                } else {
                    for window in self.window_table.values() {
                        window.request_redraw();
                    }
                }
            }
            StartCause::WaitCancelled { .. } => {}
            StartCause::Poll => {}
            StartCause::Init => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Ime(ime) => match ime {
                winit::event::Ime::Enabled => {}
                winit::event::Ime::Preedit(_, _) => {}
                winit::event::Ime::Commit(str) => {
                    self.workspace
                        .send_input(window_id, &str, self.modifiers_state)
                }
                winit::event::Ime::Disabled => {}
            },
            WindowEvent::Resized(size) => {
                self.workspace.resize(window_id, size.width, size.height);

                if let Some(window) = self.window_table.get(&window_id) {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // 将来的にこっちに乗り換える
                // self.renderer.render();

                self.workspace.render(window_id);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers_state = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                if let Some(text) = event.text_with_all_modifiers() {
                    self.workspace
                        .send_input(window_id, text, self.modifiers_state);
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
                    self.workspace
                        .send_input(window_id, name_key, self.modifiers_state);
                }

                // 装飾キーが押されてると text_with_all_modifiers() が取得できないときがあるのでその救済措置
                if let Some(text) = event.key_without_modifiers().to_text() {
                    self.workspace
                        .send_input(window_id, text, self.modifiers_state);
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
                // サーバーば起動していたら終了要求を出す
                // if is_profile_server_enabled {
                //     tx.send(()).unwrap();
                // }

                // runtime.block_on(async {
                //     profiler_server_task.await.unwrap();
                // });
            }
            _ => {}
        }
    }
}
