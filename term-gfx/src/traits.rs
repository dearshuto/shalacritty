use raw_window_handle::{DisplayHandle, WindowHandle};
use std::hash::Hash;

pub enum BufferUsage {
    IndexBuffer,
    VertexBuffer,
    UniformBuffer,
    UnorderedAccessBuffer,
    CopySource,
}

pub enum DescriptorType {
    ConstantBuffer,
    StorageBuffer,
}

pub trait IBackend {
    type RenderTargetId: Hash + Eq + Copy + Clone;
    type SemaphoreId: Hash + Eq + Copy + Clone;
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

    fn create_semaphore(&mut self, id: Self::RenderTargetId) -> Result<Self::SemaphoreId, ()>;

    fn create_pipeline<T>(
        &mut self,
        id: Self::RenderTargetId,
        shader_code_provider: T,
    ) -> Result<Self::PipelineId, ()>
    where
        T: IShaderCodeProvider;

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

    fn acquire_next_frame(
        &mut self,
        id: Self::RenderTargetId,
        semaphore_id: Self::SemaphoreId,
    ) -> Result<u32, ()>;

    fn render(
        &self,
        render_params: RenderParams<
            Self::RenderTargetId,
            Self::SemaphoreId,
            Self::PipelineId,
            Self::DescriptorSetId,
            Self::BufferId,
        >,
    );

    fn present(
        &self,
        process_index: u32,
        id: Self::RenderTargetId,
        wait_semaphore_id: Self::SemaphoreId,
    );
}

pub trait IMapHandle {
    fn write(&mut self, offset: usize, data: &[u8]);
}

pub trait IShaderCodeProvider {
    fn get_background_vertex_shader_binary(&self) -> &[u32];

    fn get_background_fragment_shader_binary(&self) -> &[u32];

    fn get_character_vertex_shader_binary(&self) -> &[u32];

    fn get_character_fragment_shader_binary(&self) -> &[u32];
}

#[derive(Debug)]
pub struct RenderParams<TRenderTargetId, TSemaporeId, TPipelineId, TDescriptorSetId, TBufferId> {
    pub process_index: u32,
    pub render_target_id: TRenderTargetId,
    pub acquire_next_frame_semaphore_id: TSemaporeId,
    pub queue_submit_signal_semaphore_id: TSemaporeId,
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
