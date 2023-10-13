#[allow(dead_code)]
pub struct Renderer;

impl Renderer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let _vertex_shader_binary = include_bytes!("rect.vs.spv");
        let _pixel_shader_binary = include_bytes!("rect.fs.spv");
        Self
    }
}
