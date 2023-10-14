use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use crate::{gfx::GlyphManager, tty::TeletypeManager, window::WindowManager};

pub struct App;

impl App {
    pub async fn run() {
        // グリフの抽出は時間がかかるので最初に処理を始める
        let mut glyph_manager = GlyphManager::new();
        let task = tokio::spawn(async move { glyph_manager.extract_alphabet().await });

        let event_loop = EventLoopBuilder::new().build();
        let instance = wgpu::Instance::default();

        let mut window_manager = WindowManager::new();
        let id = window_manager.create_window(&instance, &event_loop).await;

        let mut teletype_manager = TeletypeManager::new();
        let _tty_id = teletype_manager.create_teletype();

        // アルファベットの抽出待ち
        task.await.unwrap();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            let _queue = window_manager.try_get_queue(id).unwrap();

            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(_size),
                    ..
                } => {
                    let device = window_manager.get_device(id).unwrap();
                    let _buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: 128,
                        usage: wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: true,
                    });
                    device.poll(wgpu::Maintain::Wait);
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
