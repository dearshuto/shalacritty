use super::IGraphics;

pub struct GraphicsAsh<'a> {
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> GraphicsAsh<'a> {}

impl<'a> IGraphics<'a> for GraphicsAsh<'a> {
    type TDevice = ash::vk::Device;
    type TSurface = ash::vk::SurfaceKHR;
    type TBuffer = ash::vk::Buffer;
    type TShader = ash::vk::ShaderModule;

    fn create_device(&mut self) -> Self::TDevice {
        todo!()
    }

    fn create_surface<TWindow>(&mut self, _window: TWindow) -> Self::TSurface
    where
        TWindow: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + wgpu::WasmNotSendSync
            + 'a,
    {
        todo!()
    }

    fn crate_buffer(&mut self) -> Self::TBuffer {
        todo!()
    }

    fn create_shader(&mut self) -> Self::TShader {
        todo!()
    }
}
