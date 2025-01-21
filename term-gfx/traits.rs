use raw_window_handle::{DisplayHandle, WindowHandle};
use std::hash::Hash;

pub enum BufferUsage {
    IndexBuffer,
    VertexBuffer,
    UniformBuffer,
    UnorderedAccessBuffer,
}

pub enum DescriptorType {
    ConstantBuffer,
    StorageBuffer,
}

pub trait IBackend {
    type RenderTargetId: Hash + Eq + Copy + Clone;
    type PipelineId: Hash + Eq + Copy + Clone;
    type DescriptorSetId: Hash + Eq + Copy + Clone;
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

    fn allocate_descriptor_set(
        &mut self,
        id: Self::RenderTargetId,
        pipeline_id: Self::PipelineId,
    ) -> Result<Self::DescriptorSetId, ()>;

    fn update_descriptors(
        &mut self,
        update_descriptors_params: &UpdateDescriptorsParams<
            Self::RenderTargetId,
            Self::DescriptorSetId,
            Self::BufferId,
        >,
    );

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

    fn render(
        &self,
        render_params: RenderParams<
            Self::RenderTargetId,
            Self::PipelineId,
            Self::DescriptorSetId,
            Self::BufferId,
        >,
    );
}

pub trait IMapHandle {
    fn write(&mut self, offset: usize, data: &[u8]);
}

#[derive(Debug)]
pub struct RenderParams<TRenderTargetId, TPipelineId, TDescriptorSetId, TBufferId> {
    pub render_target_id: TRenderTargetId,
    pub pipelie_id: TPipelineId,
    pub descriptor_set_id: Option<TDescriptorSetId>,
    pub vertex_buffer_id: TBufferId,
    pub index_buffer_id: TBufferId,
    pub index_count: u32,
    pub instance_count: u32,
}

#[derive(Debug)]
pub struct UpdateBufferInfo<TBufferId> {
    pub id: TBufferId,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug)]
pub struct UpdateDescriptorsParams<TRenderTargetId, TDescriptorSetId, TBufferId> {
    pub render_target_id: TRenderTargetId,
    pub descriptor_set_id: TDescriptorSetId,
    pub buffer_info: Vec<UpdateBufferInfo<TBufferId>>,
}
