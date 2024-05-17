pub trait IRenderPlugin<'a> {
    fn register(&mut self, instance: &wgpu::Instance);

    fn resize(&mut self, width: u32, height: u32);

    #[allow(dead_code)]
    fn render(&self, render_pass: wgpu::RenderPass<'a>);
}
