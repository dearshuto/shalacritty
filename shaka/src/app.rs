use renge::ServiceRunner;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::EventLoopProxy,
    window::{Window, WindowAttributes},
};

use crate::services::{
    EventStream, GlyphExtractService, InputEventService, RenderingService, RenderingServiceParams,
    ShellService, StreamingEvent, ZellijBridgeService,
};

pub struct UserEvent {}

pub struct App {
    window: Option<Window>,
    service_runner: renge::DefaultServiceRunner,
    #[allow(unused)]
    event_loop_proxy: EventLoopProxy<UserEvent>,
    redraw_request_sender: Option<tokio::sync::mpsc::Sender<()>>,
    event_sender_bridge: Option<std::sync::mpsc::Sender<StreamingEvent>>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            window: None,
            service_runner: ServiceRunner::default(),
            event_loop_proxy: proxy,
            redraw_request_sender: None,
            event_sender_bridge: None,
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes =
            WindowAttributes::default().with_inner_size(PhysicalSize::new(1280, 960));
        let window = event_loop.create_window(window_attributes).unwrap();

        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (event_sender, event_receiver) = tokio::sync::broadcast::channel(1);
        let event_stream = EventStream::new(receiver, event_sender);
        self.service_runner.push(event_stream);
        let (event_sender_bridge, event_receiver_bridge) =
            std::sync::mpsc::channel::<StreamingEvent>();
        let _ = tokio::spawn(async move {
            tokio::task::spawn_blocking(async move || {
                while let Ok(event) = event_receiver_bridge.recv() {
                    sender.send(event).await.unwrap()
                }
            })
            .await
            .unwrap()
            .await;
        });

        // キー入力
        let (action_sender, action_receiver) = tokio::sync::mpsc::channel(1);
        let input_event_service = InputEventService::new(action_sender);
        self.service_runner
            .push_with_params(input_event_service, event_receiver);

        // シェルサービス
        let (content_sender, content_receiver) = tokio::sync::mpsc::channel(1);
        let shell_service = ShellService::new(content_sender, action_receiver);
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
        self.event_sender_bridge = Some(event_sender_bridge);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput { .. } => {
                let Some(sender) = &self.event_sender_bridge else {
                    return;
                };

                sender
                    .send(StreamingEvent {
                        window_id,
                        window_event: event,
                    })
                    .unwrap_or_default();
            }
            WindowEvent::RedrawRequested => {
                let Some(sender) = &self.redraw_request_sender else {
                    return;
                };

                let sender_cloned = sender.clone();
                tokio::spawn(async move { sender_cloned.send(()).await.unwrap() });
            }
            WindowEvent::Resized(_) => {
                let Some(sender) = &self.event_sender_bridge else {
                    return;
                };

                sender
                    .send(StreamingEvent {
                        window_id,
                        window_event: event,
                    })
                    .unwrap_or_default();
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
