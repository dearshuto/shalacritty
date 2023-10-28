use std::sync::{Arc, Mutex};

use alacritty_terminal::event_loop::Msg;
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::{
    gfx::{ContentPlotter, GlyphManager, Renderer},
    tty::TeletypeManager,
    window::WindowManager,
    ConfigService,
};

pub struct App;

impl App {
    pub async fn run() {
        let _config_service = ConfigService::new();

        // グリフの抽出は時間がかかるので最初に処理を始める
        let mut glyph_manager = GlyphManager::new();
        let task = tokio::spawn(async move {
            glyph_manager.extract_alphabet_async().await;
            glyph_manager
        });

        let event_loop = EventLoopBuilder::new().build();
        let instance = wgpu::Instance::default();

        let mut window_manager = WindowManager::new();
        let id = window_manager.create_window(&event_loop).await;
        let window = window_manager.try_get_window(id.clone()).unwrap();

        let mut teletype_manager = TeletypeManager::new();
        let (tty_id, channel) = teletype_manager.create_teletype();
        let (_tty_id, _channel) = teletype_manager.create_teletype();

        let mut renderer = Renderer::new();
        renderer.register(id.clone(), &instance, &window).await;
        let renderer = Arc::new(Mutex::new(renderer));
        let r_l = renderer.clone();
        let r_e = renderer.clone();

        let window_manager = Arc::new(Mutex::new(window_manager));

        // アルファベットの抽出待ち
        let mut glyph_manager = task.await.unwrap();

        let mut plotter = ContentPlotter::new();

        let l = window_manager.clone();
        let w = window_manager.clone();
        let _job = tokio::task::spawn(async move {
            loop {
                // 表示する要素が更新されていたら描画する要素に反映する
                if teletype_manager.is_dirty(tty_id) {
                    teletype_manager.get_content(tty_id, |c| {
                        let diff = plotter.calculate_diff(c, &mut glyph_manager);
                        r_l.lock().unwrap().update(id, diff);
                    });
                    let binding = l.lock().unwrap();
                    let window = binding.try_get_window(id).unwrap();
                    window.request_redraw();
                    teletype_manager.clear_dirty(tty_id);
                }

                std::thread::sleep(std::time::Duration::from_millis(8));
            }
        });

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    renderer
                        .lock()
                        .unwrap()
                        .resize(id.clone(), size.width, size.height);

                    let binding = w.lock().unwrap();
                    let window = binding.try_get_window(id.clone()).unwrap();
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state != ElementState::Pressed {
                            return;
                        }

                        let text = crate::convert_key_to_str(input.virtual_keycode.unwrap());
                        let mut bytes = Vec::with_capacity(text.len() + 1);
                        bytes.extend_from_slice(text.as_bytes());
                        if text.len() == 0 {
                            bytes.push(b'\x1b');
                        }
                        channel.send(Msg::Input(bytes.into()));
                    }
                    WindowEvent::CloseRequested { .. } => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                },
                Event::RedrawRequested(_) => {
                    // job.abort();
                    r_e.lock().unwrap().render(id.clone());
                }
                _ => {}
            }
        });
    }
}
