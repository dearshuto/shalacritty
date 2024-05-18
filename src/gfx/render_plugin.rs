pub trait IRenderPlugin {
    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);

    fn register(&mut self, instance: &wgpu::Instance);

    fn resize(&mut self, width: u32, height: u32);

    fn render(&self, render_target: &wgpu::TextureView, command_encoder: &mut wgpu::CommandEncoder);
}
