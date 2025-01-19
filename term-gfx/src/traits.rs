use raw_window_handle::{DisplayHandle, WindowHandle};
use std::hash::Hash;

pub enum BufferUsage {
    IndexBuffer,
    VertexBuffer,
    UniformBuffer,
    UnorderedAccessBuffer,
}

pub trait IBackend {
    type RenderTargetId: Hash + Eq + Copy + Clone;
    type PipelineId: Hash + Eq + Copy + Clone;
    type BufferId: Hash + Eq + Copy + Clone;
    type MapHandle: IMapHandle;

    fn new() -> Self;

    fn register_surface(
        &mut self,
        window_handle: WindowHandle,
        display_handle: DisplayHandle,
    ) -> Result<Self::RenderTargetId, ()>;

    fn create_pipeline(
        &mut self,
        id: Self::RenderTargetId,
        vertex_shader_spv: &[u8],
        pixel_shader_spv: &[u8],
    ) -> Result<Self::PipelineId, ()>;

    fn allocate_buffer(
        &mut self,
        id: Self::RenderTargetId,
        size: usize,
        usage: BufferUsage,
    ) -> Result<Self::BufferId, ()>;

    fn destroy_buffer(&mut self, id: Self::RenderTargetId, buffer_id: Self::BufferId);

    fn map_buffer(
        &mut self,
        id: Self::RenderTargetId,
        buffer_id: Self::BufferId,
    ) -> Result<Self::MapHandle, ()>;

    fn flush_buffer(
        &mut self,
        id: Self::RenderTargetId,
        buffer_id: Self::BufferId,
        offset: usize,
        size: usize,
    );

    fn acquire_next_image(&mut self, id: Self::RenderTargetId) -> Result<u32, ()>;

    fn render(
        &self,
        process_frame: u32,
        render_params: RenderParams<Self::RenderTargetId, Self::PipelineId, Self::BufferId>,
    );
}

pub trait IMapHandle {
    fn write(&mut self, offset: usize, data: &[u8]);
}

#[derive(Debug)]
pub struct RenderParams<TRenderTargetId, TPipelineId, TBufferId> {
    pub render_target_id: TRenderTargetId,
    pub pipelie_id: TPipelineId,
    pub vertex_buffer_id: TBufferId,
    pub index_buffer_id: TBufferId,
    pub index_count: u32,
    pub instance_count: u32,
}
