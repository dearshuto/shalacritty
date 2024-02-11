use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::WasmNotSendSync;

use super::IGraphics;

pub struct GraphicsWgpu<'a> {
    instance: wgpu::Instance,
    device: wgpu::Device,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> GraphicsWgpu<'a> {
    pub fn new() -> Self {
        todo!()
    }
}

impl<'a> IGraphics<'a> for GraphicsWgpu<'a> {
    type TDevice = wgpu::Device;
    type TSurface = wgpu::Surface<'a>;
    type TBuffer = wgpu::Buffer;
    type TShader = wgpu::ShaderModule;

    fn create_device(&mut self) -> Self::TDevice {
        todo!()
    }

    fn create_surface<TWindow>(&mut self, window: TWindow) -> Self::TSurface
    where
        TWindow: HasWindowHandle + HasDisplayHandle + WasmNotSendSync + 'a,
    {
        let surface = self.instance.create_surface(window).unwrap();
        // let adapter = instance
        //     .request_adapter(&wgpu::RequestAdapterOptions {
        //         power_preference: wgpu::PowerPreference::default(),
        //         force_fallback_adapter: false,
        //         compatible_surface: Some(&surface),
        //     })
        //     .await
        //     .unwrap();
        surface
    }

    fn crate_buffer(&mut self) -> Self::TBuffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        buffer
    }

    fn create_shader(&mut self) -> Self::TShader {
        todo!()
    }
}
