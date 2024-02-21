mod content_plotter;
mod detail;
mod glyph_manager;
mod glyph_writer;
mod graphics_ash;
mod graphics_wgpu;
mod renderer;

pub use content_plotter::ContentPlotter;
pub use glyph_manager::GlyphManager;
pub use glyph_writer::GlyphWriter;
pub use graphics_wgpu::GraphicsWgpu;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
pub use renderer::Renderer;
use wgpu::WasmNotSendSync;

pub trait IGraphics<'a> {
    type TDevice;
    type TSurface;
    type TBuffer;
    type TShader;

    fn create_device(&mut self) -> Self::TDevice;

    fn create_surface<TWindow>(&mut self, window: TWindow) -> Self::TSurface
    where
        TWindow: HasWindowHandle + HasDisplayHandle + WasmNotSendSync + 'a;

    fn crate_buffer(&mut self) -> Self::TBuffer;

    fn create_shader(&mut self) -> Self::TShader;
}
