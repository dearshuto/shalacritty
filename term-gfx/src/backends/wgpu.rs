#[allow(unused)]
pub struct BackendWgpu<'a> {
    surface: wgpu::Surface<'a>,
}

#[allow(unused)]
impl<'a> BackendWgpu<'a> {
    pub fn new(window: impl Into<wgpu::SurfaceTarget<'a>>) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).unwrap();

        Self { surface }
    }
}
