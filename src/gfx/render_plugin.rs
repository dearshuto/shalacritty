pub struct UpdateParams<'a, T> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub user_data: &'a T,
}

impl<'a, T> IRenderPluginUpdateParams for UpdateParams<'a, T> {
    type TUserData = T;

    fn device(&self) -> &wgpu::Device {
        self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        self.queue
    }

    fn user_data(&self) -> &T {
        self.user_data
    }
}

pub trait IRenderPluginUpdateParams {
    type TUserData;

    fn device(&self) -> &wgpu::Device;

    fn queue(&self) -> &wgpu::Queue;

    fn user_data(&self) -> &Self::TUserData;
}

pub trait IRenderPlugin {
    type TRenderPluginUpdateParams: IRenderPluginUpdateParams;

    fn update(&mut self, update_params: &Self::TRenderPluginUpdateParams);

    fn register(&mut self, instance: &wgpu::Instance);

    fn resize(&mut self, width: u32, height: u32);

    fn render(&self, render_target: &wgpu::TextureView, command_encoder: &mut wgpu::CommandEncoder);
}
