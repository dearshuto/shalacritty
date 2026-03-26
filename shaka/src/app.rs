use renge::ServiceRunner;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::EventLoopProxy,
    window::{Window, WindowAttributes},
};

use crate::services::{
    GlyphExtractService, InputEventService, RenderingService, RenderingServiceParams, ShellService,
    ZellijBridgeService,
};

pub struct UserEvent {}

pub struct App {
    window: Option<Window>,
    service_runner: renge::DefaultServiceRunner,
    #[allow(unused)]
    event_loop_proxy: EventLoopProxy<UserEvent>,
    redraw_request_sender: Option<tokio::sync::mpsc::Sender<()>>,
    key_event_sender: Option<std::sync::mpsc::Sender<KeyEvent>>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            window: None,
            service_runner: ServiceRunner::default(),
            event_loop_proxy: proxy,
            redraw_request_sender: None,
            key_event_sender: None,
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_inner_size(PhysicalSize::new(1280, 960))
            .with_resizable(false);
        let window = event_loop.create_window(window_attributes).unwrap();

        // キー入力
        let (key_event_sender, key_event_receiver) = std::sync::mpsc::channel();
        let (action_sender, _action_receiver) = tokio::sync::mpsc::channel(1);
        let _input_event_service_handle = tokio::spawn(async move {
            tokio::task::spawn_blocking(async || {
                let input_event_service = InputEventService::new(key_event_receiver, action_sender);
                input_event_service.serve().await;
            })
            .await
            .unwrap()
            .await;
        });

        // シェルサービス
        let (content_sender, content_receiver) = tokio::sync::mpsc::channel(1);
        let shell_service = ShellService::new(content_sender);
        self.service_runner.push(shell_service);

        // グリフ抽出サービス
        // グリフのラスタライズが Send ではないので特定のスレッドに保持させてチャンネルでやりとりする
        // CPU を専有しないように spawn_blocking で起動する
        let (request_sender, request_receiver) = std::sync::mpsc::channel();
        let _glyph_service_handle = tokio::spawn(async move {
            tokio::task::spawn_blocking(|| {
                let glyph_extract_service = GlyphExtractService::new(request_receiver);
                glyph_extract_service.serve();
            })
            .await
            .unwrap();
        });

        let rendering_service = RenderingService::new(&window);
        let (redraw_request_sender, redraw_request_receiver) = tokio::sync::mpsc::channel(1);
        // ラスタライズ要求は大量に来る可能性があるのである程度のバッファーをもたせた
        let (glyph_service_adapter_sender, mut glyph_service_adapter_receiver) =
            tokio::sync::mpsc::channel(64);
        let params = RenderingServiceParams {
            receiver: redraw_request_receiver,
            content_receiver,
            glyph_request_sender: glyph_service_adapter_sender,
        };
        self.service_runner
            .push_with_params(rendering_service, params);

        let zellij_bridge_service = ZellijBridgeService::new();
        self.service_runner.push(zellij_bridge_service);

        // 同期的に動くグリフのラスタライズと非同期のサービスの変換
        let _ = tokio::spawn(async move {
            while let Some(request) = glyph_service_adapter_receiver.recv().await {
                request_sender.send(request).unwrap();
            }
        });

        window.request_redraw();
        self.window = Some(window);
        self.redraw_request_sender = Some(redraw_request_sender);
        self.key_event_sender = Some(key_event_sender);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput {
                #[allow(unused)]
                device_id,
                event,
                #[allow(unused)]
                is_synthetic,
            } => {
                let Some(sender) = &self.key_event_sender else {
                    return;
                };

                sender.send(event).unwrap_or_default();
            }
            WindowEvent::RedrawRequested => {
                let Some(sender) = &self.redraw_request_sender else {
                    return;
                };

                let sender_cloned = sender.clone();
                tokio::spawn(async move { sender_cloned.send(()).await.unwrap() });
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, _event: UserEvent) {
        let Some(window) = &self.window else {
            return;
        };

        window.request_redraw();
    }
}
