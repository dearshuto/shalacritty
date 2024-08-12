pub struct UpdateParams<'a, T> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub user_data: &'a T,
}

impl<'a, T> UpdateParams<'a, T> {
    pub fn device(&self) -> &wgpu::Device {
        self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.queue
    }

    pub fn user_data(&self) -> &T {
        self.user_data
    }
}

pub trait IRenderPlugin {
    type UserData;

    fn update(&mut self, update_params: &UpdateParams<'_, Self::UserData>);

    fn register(&mut self, instance: &wgpu::Instance);

    fn resize(&mut self, width: u32, height: u32);

    fn render(&self, render_target: &wgpu::TextureView, command_encoder: &mut wgpu::CommandEncoder);
}
