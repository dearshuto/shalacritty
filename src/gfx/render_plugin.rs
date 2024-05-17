pub trait IRenderPlugin {
    fn register(&mut self, instance: &wgpu::Instance);

    fn resize(&mut self, width: u32, height: u32);

    fn render(&self, render_pass: wgpu::RenderPass);
}
