use alacritty_terminal::event_loop::Msg;
use winit::{
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::{
    gfx::{ContentPlotter, GlyphManager, Renderer},
    tty::TeletypeManager,
    window::WindowManager,
};

pub struct App;

impl App {
    pub async fn run() {
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

        let mut renderer = Renderer::new();
        renderer.register(id.clone(), &instance, &window).await;

        // アルファベットの抽出待ち
        let glyph_manager = task.await.unwrap();

        let mut plotter = ContentPlotter::new();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            // 表示する要素が更新されていたら描画する要素に反映する
            if teletype_manager.is_dirty(tty_id) {
                teletype_manager.get_content(tty_id, |c| {
                    let diff = plotter.calculate_diff(c, &glyph_manager);
                    renderer.update(id.clone(), diff);
                });
                let window = window_manager.try_get_window(id.clone()).unwrap();
                window.request_redraw();
                teletype_manager.clear_dirty(tty_id);
            }

            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    renderer.resize(id.clone(), size.width, size.height);

                    let window = window_manager.try_get_window(id.clone()).unwrap();
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state != ElementState::Pressed {
                            return;
                        }

                        let text = match input.virtual_keycode.unwrap() {
                            VirtualKeyCode::B => "b",
                            VirtualKeyCode::D => "d",
                            VirtualKeyCode::E => "e",
                            VirtualKeyCode::L => "l",
                            VirtualKeyCode::M => "m",
                            VirtualKeyCode::N => "n",
                            VirtualKeyCode::P => "p",
                            VirtualKeyCode::Q => "q",
                            VirtualKeyCode::S => "s",
                            VirtualKeyCode::T => "t",
                            VirtualKeyCode::U => "u",
                            VirtualKeyCode::W => "w",
                            VirtualKeyCode::Return => "\n",
                            _ => "",
                        };
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
                    renderer.render(id.clone());
                }
                _ => {}
            }
        });
    }
}
