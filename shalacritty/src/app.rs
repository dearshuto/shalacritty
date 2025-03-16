use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use profiler_core::IServerBackend;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use term_gfx::IBackend;
use tokio::{sync::oneshot, task::JoinHandle};

use winit::{
    application::ApplicationHandler,
    event::{ElementState, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{ModifiersState, NamedKey},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
};

use crate::{
    config::ConfigServiceEx,
    detail::{
        ContentPlotService, ImageCacheEx, PollingEventService, RenderingService, ShellService,
        WindowSizeSendService,
    },
    workspace::{IWorkspaceCallback, Workspace},
};

/// WindowSizeChangeEvent ─┬─────────────────────────────────┐
///                        │                                 │
/// InputEvent ─────────┐  │                                 │
///                     │  │                                 │
///                     v  v                                 v
/// ConfigService ───> ShellService ───> PlotService ───> RenderService
///      |                                   ^               ^
///      ├───────────> ImageCacheService ────┘               │
///      └───────────────────────────────────────────────────┘
pub struct App<'a, TBackend>
where
    TBackend: term_gfx::IBackend,
{
    instance: Option<super::config::Instance>,

    window_table: HashMap<winit::window::WindowId, winit::window::Window>,

    #[allow(unused)]
    rendering_service: RenderingService<'a>,
    window_created_sender: Option<std::sync::mpsc::Sender<WindowCreatedEventArgs>>,
    window_size_sender: Option<std::sync::mpsc::Sender<WindowSizeChangedEventArgs>>,
    input_sender: Option<std::sync::mpsc::Sender<KeyboadInputEventArgs>>,

    workspace: Arc<Mutex<Workspace<'static, ServerBackend>>>,
    renderer: term_gfx::Renderer<TBackend>,
    modifiers_state: ModifiersState,
    profiler_kill_sender: Option<oneshot::Sender<()>>,

    service_tasks: Vec<JoinHandle<()>>,

    polling_close_sender: Option<std::sync::mpsc::Sender<()>>,

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

        // ポーリングサービス
        let (polling_close_sender, polling_close_receiver) = std::sync::mpsc::channel();
        let mut polling_event_service = PollingEventService::new(polling_close_receiver);

        // 設定ファイル監視サービス
        let (config_watch_instance, config_receiver) = super::config::watch();
        let mut config_service = ConfigServiceEx::new(config_receiver.clone());

        let (window_created_sender, window_created_receiver) = std::sync::mpsc::channel();
        let (window_size_sender, window_size_receiver) = std::sync::mpsc::channel();

        // ウィンドウサイズサービス
        let mut window_size_send_service = WindowSizeSendService::new(
            window_created_receiver,
            window_size_receiver,
            polling_event_service.listen(),
        );

        // シェル管理サービス
        let (input_sender, input_receiver) = std::sync::mpsc::channel();
        let shell_service = ShellService::new(
            config_service.listen(),
            window_size_send_service.listen(),
            input_receiver,
            polling_event_service.listen(),
        );
        let shell_service_task = runtime.spawn(async move {
            shell_service.serve().await;
        });

        // 表示コンテンツの座標を計算するサービス
        let content_plot_service = ContentPlotService::new(config_service.listen());
        let content_plot_service_task = runtime.spawn(async move {
            content_plot_service.serve().await;
        });

        // 画像キャッシュサービス
        let image_cache_service = ImageCacheEx::new(runtime.clone(), config_service.listen());
        let image_cache_service_task = runtime.spawn(async move {
            image_cache_service.serve().await;
        });

        // 描画サービス
        let (_window_created_sender, window_created_receiver) = tokio::sync::mpsc::channel(1);
        let (_redraw_requested_sender, redraw_requested_receiver) = tokio::sync::mpsc::channel(1);

        let rendering_service = RenderingService::new(
            config_service.listen(),
            window_created_receiver,
            window_size_send_service.listen(),
            redraw_requested_receiver,
        );

        let server_backend = ServerBackend::new();
        let server_backend_local = server_backend.clone();

        let (tx, rx) = oneshot::channel::<()>();
        let profiler_server_task = runtime.spawn(async move {
            if is_profile_server_enabled {
                profiler_core::Server::serve(([0, 0, 0, 0], 3030), server_backend_local, rx).await;
            }
        });

        let workspace = Arc::new(Mutex::new(Workspace::new_with_callback(
            runtime.clone(),
            config_receiver,
            server_backend,
        )));

        // ウィンドウサイズの変更を非同期に Workspace に反映するタスク
        // 互換用の実装で、将来的に Workspace は解体予定
        let mut window_size_receiver = window_size_send_service.listen();
        let workspace_local = workspace.clone();
        let workspace_resize_task = runtime.spawn(async move {
            while let Some(args) = window_size_receiver.recv().await {
                workspace_local
                    .lock()
                    .unwrap()
                    .resize(args.id, args.width, args.height);
            }
        });

        let window_size_send_service = runtime.spawn(async move {
            window_size_send_service.serve().await;
        });

        // 設定ファイルサービスタスク
        // タスク化と同時にムーブするので他のサービスたちが購読を開始してから記述している
        let config_service_task = runtime.spawn(async move {
            config_service.serve().await;
        });

        let polling_event_service_task = runtime.spawn(async move {
            polling_event_service.serve().await;
        });

        let service_tasks = vec![
            polling_event_service_task,
            config_service_task,
            window_size_send_service,
            shell_service_task,
            content_plot_service_task,
            image_cache_service_task,
            profiler_server_task,
            workspace_resize_task,
        ];

        Self {
            instance: Some(config_watch_instance),
            runtime,
            window_table: HashMap::default(),
            input_sender: Some(input_sender),
            window_created_sender: Some(window_created_sender),
            window_size_sender: Some(window_size_sender),
            workspace,
            renderer,
            polling_close_sender: Some(polling_close_sender),
            modifiers_state: ModifiersState::default(),
            profiler_kill_sender: Some(tx),
            rendering_service,
            service_tasks,
        }
    }

    pub fn run(renderer: term_gfx::Renderer<TBackend>, is_profile_server_enabled: bool) {
        let event_loop = EventLoop::builder().build().unwrap();

        let mut app = Self::new(renderer, is_profile_server_enabled);
        event_loop.run_app(&mut app).unwrap();
    }
}

impl<'a, TBackend> Drop for App<'a, TBackend>
where
    TBackend: IBackend,
{
    fn drop(&mut self) {
        // 終了を通知して起動したサービスを終了させる
        // channel に紐づいたサービスはインスタンスを破棄することで止める
        self.window_created_sender = None;
        self.window_size_sender = None;
        self.instance = None;
        self.polling_close_sender = None;
        self.input_sender = None;

        // プロファイルサーバーが起動していたら終了する
        // MEMO: サービスの終了処理と統一したい
        let mut kill_server_sender = None;
        std::mem::swap(&mut kill_server_sender, &mut self.profiler_kill_sender);
        if let Some(sender) = kill_server_sender {
            sender.send(()).unwrap_or_default();
        }

        // サービスの終了待ち
        let mut service_tasks = Vec::default();
        std::mem::swap(&mut self.service_tasks, &mut service_tasks);
        self.runtime.block_on(async move {
            futures::future::join_all(service_tasks).await;
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
                .lock()
                .unwrap()
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

        // TODO: イベント駆動方式に載せ替える
        // let args = WindowCreatedEventArgs { id, window };
        // self.window_created_sender
        //     .as_ref()
        //     .unwrap()
        //     .send(args)
        //     .unwrap();

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

                if let Ok(mut workspace) = self.workspace.lock() {
                    workspace.update(*id, window.inner_size().width, window.inner_size().height);
                    if workspace.is_empty() {
                        event_loop.exit();
                    } else {
                        for window in self.window_table.values() {
                            window.request_redraw();
                        }
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
                        .lock()
                        .unwrap()
                        .send_input(window_id, &str, self.modifiers_state)
                }
                winit::event::Ime::Disabled => {}
            },
            WindowEvent::Resized(size) => {
                // 通知
                self.window_size_sender
                    .as_ref()
                    .unwrap()
                    .send(WindowSizeChangedEventArgs {
                        id: window_id,
                        width: size.width,
                        height: size.height,
                    })
                    .unwrap_or_default();

                if let Some(window) = self.window_table.get(&window_id) {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // 将来的にこっちに乗り換える
                // self.renderer.render();

                self.workspace.lock().unwrap().render(window_id);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers_state = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                // 通知
                if let Some(sender) = &self.input_sender {
                    let args = KeyboadInputEventArgs {
                        id: window_id,
                        event: event.clone(),
                    };
                    sender.send(args).unwrap();
                }

                if let Some(text) = event.text_with_all_modifiers() {
                    self.workspace.lock().unwrap().send_input(
                        window_id,
                        text,
                        self.modifiers_state,
                    );
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
                    self.workspace.lock().unwrap().send_input(
                        window_id,
                        name_key,
                        self.modifiers_state,
                    );
                }

                // 装飾キーが押されてると text_with_all_modifiers() が取得できないときがあるのでその救済措置
                if let Some(text) = event.key_without_modifiers().to_text() {
                    self.workspace.lock().unwrap().send_input(
                        window_id,
                        text,
                        self.modifiers_state,
                    );
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }
}

pub struct KeyboadInputEventArgs {
    pub id: winit::window::WindowId,
    pub event: winit::event::KeyEvent,
}

pub struct WindowCreatedEventArgs {
    pub id: winit::window::WindowId,
    pub window: winit::window::Window,
}

#[derive(Clone)]
pub struct WindowSizeChangedEventArgs {
    pub id: winit::window::WindowId,
    pub width: u32,
    pub height: u32,
}
