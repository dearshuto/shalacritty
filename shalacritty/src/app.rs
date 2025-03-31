use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use profiler_core::IServerBackend;
use term_gfx::IBackend;
use tokio::sync::oneshot;

use tracing::instrument;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::WindowId,
};

use crate::{
    config::ConfigServiceEx,
    detail::{
        ContentPlotService, GlyphExtractService, ImageCacheEx, PollingEventService,
        RenderingService, ShellService, WindowSizeSendService, WorkspaceUpdateServiceTentative,
    },
    workspace::{Action, IWorkspaceCallback, Workspace},
};

struct Instance {
    instance: Option<super::config::Instance>,

    workspace: Arc<Mutex<Workspace<'static, ServerBackend>>>,

    window_created_sender: Option<std::sync::mpsc::Sender<WindowCreatedEventArgs>>,
    window_size_sender: Option<std::sync::mpsc::Sender<WindowSizeChangedEventArgs>>,
    input_sender: Option<std::sync::mpsc::Sender<KeyboadInputEventArgs>>,

    // renderer: term_gfx::Renderer<TBackend>,
    profiler_kill_sender: Option<oneshot::Sender<()>>,

    redraw_requested_sender: Option<tokio::sync::mpsc::Sender<WindowId>>,

    polling_close_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

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
    runtime: Arc<tokio::runtime::Runtime>,

    modifiers_state: ModifiersState,

    instance: Option<Instance>,

    is_profile_server_enabled: bool,

    proxy: EventLoopProxy<UserEvent>,

    window_table: HashMap<winit::window::WindowId, Arc<winit::window::Window>>,

    _marker: std::marker::PhantomData<&'a TBackend>,
}

impl<'a, TBackend> App<'a, TBackend>
where
    TBackend: term_gfx::IBackend,
{
    pub fn run(_renderer: term_gfx::Renderer<TBackend>, is_profile_server_enabled: bool) {
        // 任意のタイミングで終了したいので Proxy 経由でイベントを発行したい
        let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_time()
                .build()
                .unwrap(),
        );

        let mut app = Self {
            runtime,
            modifiers_state: Default::default(),
            instance: None,
            is_profile_server_enabled,
            proxy: event_loop.create_proxy(),
            window_table: HashMap::default(),
            _marker: std::marker::PhantomData,
        };

        event_loop.run_app(&mut app).unwrap();
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // 終了を通知して起動したサービスを終了させる
        // channel に紐づいたサービスはインスタンスを破棄することで止める
        self.window_created_sender = None;
        self.window_size_sender = None;
        self.instance = None;
        self.input_sender = None;
        self.redraw_requested_sender = None;

        // ポーリングの終了要求
        let mut sender = None;
        std::mem::swap(&mut sender, &mut self.polling_close_sender);
        sender.unwrap().send(()).unwrap();

        // プロファイルサーバーが起動していたら終了する
        // MEMO: サービスの終了処理と統一したい
        let mut kill_server_sender = None;
        std::mem::swap(&mut kill_server_sender, &mut self.profiler_kill_sender);
        if let Some(sender) = kill_server_sender {
            sender.send(()).unwrap_or_default();
        }
    }
}

impl<'a, TBackend: IBackend> std::fmt::Debug for App<'a, TBackend> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("App")?;
        std::fmt::Result::Ok(())
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

impl<'a, TBackend> ApplicationHandler<UserEvent> for App<'a, TBackend>
where
    TBackend: term_gfx::IBackend,
{
    #[instrument]
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

        let window = Arc::new(window);

        // ポーリングサービス
        let (polling_close_sender, polling_close_receiver) = tokio::sync::oneshot::channel();
        let mut polling_event_service = PollingEventService::new(polling_close_receiver);

        // 設定ファイル監視サービス
        let (config_watch_instance, config_receiver) = super::config::watch();
        let mut config_service = ConfigServiceEx::new(config_receiver.clone());

        let (window_created_sender, window_created_receiver) = std::sync::mpsc::channel();
        let (window_size_sender, window_size_receiver) = std::sync::mpsc::channel();

        // ウィンドウサイズサービス
        let mut window_size_send_service =
            WindowSizeSendService::new(window_size_receiver, polling_event_service.listen());

        // シェル管理サービス
        let (input_sender, input_receiver) = std::sync::mpsc::channel();
        let (mut shell_service, content_receiver) = ShellService::new(
            config_service.listen(),
            window_created_receiver,
            window_size_send_service.listen(),
            input_receiver,
            polling_event_service.listen(),
        );

        // グリフ抽出サービス
        let glyph_extract_service =
            GlyphExtractService::new(config_service.listen(), shell_service.listen_string());
        let glyph_container = glyph_extract_service.share_glyph_container();
        let _ = tokio::task::Builder::new()
            .name("GlyphExtractService")
            .spawn_on(
                async move {
                    glyph_extract_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        // 表示コンテンツの座標を計算するサービス
        let (content_plot_service, mut diff_receiver) =
            ContentPlotService::new(content_receiver, glyph_container);
        let _ = tokio::task::Builder::new()
            .name("ContentPlotService")
            .spawn_on(
                async move {
                    content_plot_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        // 画像キャッシュサービス
        let image_cache_service = ImageCacheEx::new(self.runtime.clone(), config_service.listen());
        let _ = tokio::task::Builder::new()
            .name("ImageCacheService")
            .spawn_on(
                async move {
                    image_cache_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        // 描画サービス
        let (redraw_requested_sender, redraw_requested_receiver) = tokio::sync::mpsc::channel(1);
        let rendering_service = RenderingService::new(
            window.clone(),
            config_service.listen(),
            window_size_send_service.listen(),
            redraw_requested_receiver,
        );

        let _ = tokio::task::Builder::new()
            .name("RenderingServiceTask")
            .spawn_on(
                async move {
                    rendering_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        let server_backend = ServerBackend::new();
        let server_backend_local = server_backend.clone();
        let is_profile_server_enabled_local = self.is_profile_server_enabled;

        let (tx, rx) = oneshot::channel::<()>();
        let _ = tokio::task::Builder::new()
            .name("ProfilerServerTask")
            .spawn_on(
                async move {
                    if is_profile_server_enabled_local {
                        profiler_core::Server::serve(
                            ([0, 0, 0, 0], 3030),
                            server_backend_local,
                            rx,
                        )
                        .await;
                    }
                },
                self.runtime.handle(),
            )
            .unwrap();

        // 差分を Debug 出力
        let _ = tokio::task::Builder::new()
            .name("DebugPrintTask")
            .spawn_on(
                async move {
                    if cfg!(debug_assertions) {
                        while let Some(diff) = diff_receiver.recv().await {
                            println!("==================");
                            println!("{:?}", diff.contents);
                        }
                    } else {
                        // Release 版ではなにもしない
                    }
                },
                self.runtime.handle(),
            )
            .unwrap();

        let workspace = Arc::new(Mutex::new(Workspace::new_with_callback(
            self.runtime.clone(),
            config_receiver,
            self.proxy.clone(),
            server_backend,
        )));

        // ウィンドウサイズの変更を非同期に Workspace に反映するタスク
        // 互換用の実装で、将来的に Workspace は解体予定
        let mut window_size_receiver = window_size_send_service.listen();
        let workspace_local = workspace.clone();
        let _ = tokio::task::Builder::new()
            .name("WorkspaceResizeTask")
            .spawn_on(
                async move {
                    while let Some(args) = window_size_receiver.recv().await {
                        workspace_local
                            .lock()
                            .unwrap()
                            .resize(args.id, args.width, args.height);
                    }
                },
                self.runtime.handle(),
            )
            .unwrap();

        // Workspace の更新処理を非同期に実行するサービス
        let workspace_update_service = WorkspaceUpdateServiceTentative::new(
            workspace.clone(),
            polling_event_service.listen(),
            window_size_send_service.listen(),
        );
        let _ = tokio::task::Builder::new()
            .name("WorkspaceUpdateService")
            .spawn_on(
                async move {
                    workspace_update_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        let _ = tokio::task::Builder::new()
            .name("WindowSizeSendService")
            .spawn_on(
                async move {
                    window_size_send_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        // 設定ファイルサービスタスク
        // タスク化と同時にムーブするので他のサービスたちが購読を開始してから記述している
        let _ = tokio::task::Builder::new()
            .name("ConfigService")
            .spawn_on(
                async move {
                    config_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        let _ = tokio::task::Builder::new()
            .name("PollintEventService")
            .spawn_on(
                async move {
                    polling_event_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        // シェル管理サービスタスク
        let _ = tokio::task::Builder::new()
            .name("ShellService")
            .spawn_on(
                async move {
                    shell_service.serve().await;
                },
                self.runtime.handle(),
            )
            .unwrap();

        self.runtime.block_on(async {
            workspace
                .lock()
                .unwrap()
                .assign_window(id, &window, width, height)
                .await;
        });

        // 通知
        // MEMO: Window インスタンスの管理もサービス化した方が良い？
        let args = WindowCreatedEventArgs {
            id,
            window: Arc::clone(&window),
        };
        window_created_sender.send(args).unwrap();

        self.window_table.insert(id, window);

        let instance = Instance {
            instance: Some(config_watch_instance),
            input_sender: Some(input_sender),
            window_created_sender: Some(window_created_sender),
            window_size_sender: Some(window_size_sender),
            workspace,
            polling_close_sender: Some(polling_close_sender),
            redraw_requested_sender: Some(redraw_requested_sender),
            profiler_kill_sender: Some(tx),
        };
        self.instance = Some(instance);
    }

    #[instrument]
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
                    let Some(instance) = &self.instance else {
                        return;
                    };

                    instance.workspace.lock().unwrap().send_input(
                        window_id,
                        &str,
                        self.modifiers_state,
                    )
                }
                winit::event::Ime::Disabled => {}
            },
            WindowEvent::Resized(size) => {
                let Some(instance) = &self.instance else {
                    return;
                };
                // 通知

                instance
                    .window_size_sender
                    .as_ref()
                    .unwrap()
                    .send(WindowSizeChangedEventArgs {
                        id: window_id,
                        width: size.width,
                        height: size.height,
                    })
                    .unwrap_or_default();
            }
            WindowEvent::RedrawRequested => {
                let Some(instance) = &self.instance else {
                    return;
                };
                instance.workspace.lock().unwrap().render(window_id);

                // 将来的にこっちに乗り換える
                // instance
                //     .redraw_requested_sender
                //     .as_ref()
                //     .unwrap()
                //     .blocking_send(window_id)
                //     .unwrap();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers_state = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let Some(instance) = &self.instance else {
                    return;
                };

                if event.state != ElementState::Pressed {
                    return;
                }

                // 通知
                if let Some(sender) = &instance.input_sender {
                    let args = KeyboadInputEventArgs {
                        id: window_id,
                        event: event.clone(),
                        state: self.modifiers_state,
                    };
                    sender.send(args).unwrap();
                }

                let text = event.text_with_all_modifiers().unwrap_or_default();
                instance.workspace.lock().unwrap().send_input(
                    window_id,
                    text,
                    self.modifiers_state,
                );
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Exit => event_loop.exit(),
            UserEvent::RequestRedraw => {
                for window in self.window_table.values() {
                    window.request_redraw();
                }
            }
        };
    }
}

#[derive(Debug)]
pub enum UserEvent {
    Exit,
    RequestRedraw,
}

pub struct KeyboadInputEventArgs {
    pub id: winit::window::WindowId,
    pub event: winit::event::KeyEvent,
    pub state: ModifiersState,
}

pub struct WindowCreatedEventArgs {
    pub id: winit::window::WindowId,
    pub window: Arc<winit::window::Window>,
}

#[derive(Debug, Clone)]
pub struct WindowSizeChangedEventArgs {
    pub id: winit::window::WindowId,
    pub width: u32,
    pub height: u32,
}

pub fn detect_action(text_with_all_modifiers: &str, modifier_state: ModifiersState) -> Action {
    if modifier_state.contains(ModifiersState::CONTROL)
        && modifier_state.contains(ModifiersState::ALT)
    {
        return Action::DumpDebugInfo;
    }

    // ペースト
    if (modifier_state.contains(ModifiersState::CONTROL)
        || modifier_state.contains(ModifiersState::SUPER))
        && modifier_state.contains(ModifiersState::SHIFT)
        && text_with_all_modifiers == "v"
    {
        return Action::Paste;
    }

    // Ctrl+<1~4>
    for tab_number in 1..=4 {
        if modifier_state.contains(ModifiersState::CONTROL)
            && text_with_all_modifiers == tab_number.to_string()
        {
            // インデックスとしては 0 始まりなので -1 しておく
            return Action::ActivateTab(tab_number - 1);
        }
    }

    //===============================================================
    // バイト表現では制御文字と区別できない入力は装飾キーの存在をチェックする

    // Ctrl+n で操作対象のシェルを変更
    if modifier_state.contains(ModifiersState::CONTROL)
        && text_with_all_modifiers == String::from_utf8(vec![14]).unwrap()
    {
        return Action::ActivateNextTile;
    }

    // Ctrl+h で画面分割
    // バイト表現では Backspace の制御文字と区別できないので装飾キーの存在をチェックする
    if modifier_state.contains(ModifiersState::CONTROL)
        && text_with_all_modifiers == String::from_utf8(vec![8]).unwrap()
    {
        return Action::SplitHorizontal;
    }

    // Ctrl+h で画面分割
    // TODO: これも装飾文字の存在をチェックした方がよい
    if text_with_all_modifiers == String::from_utf8(vec![20]).unwrap() {
        return Action::NewTab;
    }
    //===============================================================

    Action::Input(text_with_all_modifiers)
}
