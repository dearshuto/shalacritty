use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::{
    gfx::{ContentPlotter, GlyphManager, GlyphWriter, Renderer},
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

        let mut plotter = ContentPlotter::new();

        // 抽出したグリフを配置したテクスチャーを作成
        // 最終的には必要になったグリフを動的に生成しつつテクスチャーを更新するようにしたい
        let mut glyph_writer = GlyphWriter::new();
        let glyph_image_patches = glyph_writer.execute(&vec!['a', 'b', 'c'], &glyph_manager);
        for patch in glyph_image_patches {
            // TODO: ここで GPU に送る
            let _ = patch.pixels();
            let _ = patch.width();
            let _ = patch.height();
        }
        let _ = glyph_writer.get_clip_rect('a');

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            // 表示する要素が更新されていたら描画する要素に反映する
            if teletype_manager.is_dirty(tty_id) {
                teletype_manager.get_content(tty_id, |c| {
                    let diff = plotter.calculate_diff(c, &glyph_manager);
                    renderer.update(id.clone(), diff);
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
