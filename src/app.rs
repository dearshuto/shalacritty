use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

#[allow(unused_imports)]
use crate::window::WindowManager;

pub struct App;

impl App {
    pub async fn run() {
        let event_loop = EventLoopBuilder::new().build();
        let instance = wgpu::Instance::default();

        let mut window_manager = WindowManager::new();
        let id = window_manager.create_window(&instance, &event_loop).await;

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
