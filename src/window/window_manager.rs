use std::collections::HashMap;

use winit::{
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder},
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub struct WindowId {
    id: uuid::Uuid,
}

impl WindowId {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
        }
    }
}

pub struct WindowManager {
    window_table: HashMap<WindowId, Window>,
    device_table: HashMap<WindowId, wgpu::Device>,
    queue_table: HashMap<WindowId, wgpu::Queue>,
}

impl WindowManager {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            window_table: Default::default(),
            device_table: Default::default(),
            queue_table: Default::default(),
        }
    }

    pub async fn create_window<T>(
        &mut self,
        instance: &wgpu::Instance,
        event_loop: &EventLoopWindowTarget<T>,
    ) -> WindowId {
        let window = WindowBuilder::new().build(event_loop).unwrap();
        let id = WindowId::new();
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .unwrap();

        self.window_table.insert(id.clone(), window);
        self.device_table.insert(id.clone(), device);
        self.queue_table.insert(id.clone(), queue);
        id
    }

    pub fn get_device(&self, id: WindowId) -> Option<&wgpu::Device> {
        self.device_table.get(&id)
    }

    pub fn try_get_queue(&self, id: WindowId) -> Option<&wgpu::Queue> {
        self.queue_table.get(&id)
    }
}
