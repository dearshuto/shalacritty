use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::{
    gfx::{GlyphManager, Renderer},
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
        let tty_id = teletype_manager.create_teletype();

        let mut renderer = Renderer::new();
        renderer.register(id.clone(), &instance, &window).await;

        // アルファベットの抽出待ち
        let glyph_manager = task.await.unwrap();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            // 表示する要素が更新されていたら描画する要素に反映する
            if teletype_manager.is_dirty(tty_id) {
                teletype_manager.get_content(tty_id, |c| {
                    println!("redraw required: {}", c.display_iter.count());
                });
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
                Event::RedrawRequested(_) => {
                    let glyph_f = glyph_manager.get_rasterized_glyph('F');
                    renderer.update(id.clone(), glyph_f);
                    renderer.render(id.clone());
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }
        });
    }
}
