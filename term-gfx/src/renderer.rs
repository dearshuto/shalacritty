use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use raw_window_handle::{DisplayHandle, WindowHandle};

use crate::{
    backends::BackendVk,
    traits::{
        BufferDescriptorType, BufferUsage, IMapHandle, RenderParams, UpdateBufferInfo,
        UpdateDescriptorsParams,
    },
    IBackend,
};

#[repr(C)]
#[derive(Debug, Pod, Copy, Clone, Zeroable)]
pub struct BackgroundView {
    pub texcoord_transform0: [f32; 4],
    pub texcoord_transform1: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Pod, Copy, Clone, Zeroable)]
pub struct CharacterData {
    pub transform0: [f32; 4],
    pub transform1: [f32; 4],
    pub fore_ground_color: [f32; 4],
    pub uv_bl: [f32; 2],
    pub uv_tr: [f32; 2],
}

struct Instance<TBackend: IBackend> {
    pipeline_id: TBackend::PipelineId,
    descriptor_set_id: Option<TBackend::DescriptorSetId>,
    vertex_buffer_id: TBackend::BufferId,
    index_buffer_id: TBackend::BufferId,
}

struct SyncSemaphores<TBackend: IBackend> {
    acquire_next_image_semaphore_id: TBackend::SemaphoreId,
    background_render_signal_semaphore_id: TBackend::SemaphoreId,
    character_render_signal_semaphore_id: TBackend::SemaphoreId,
}

pub struct Renderer<TBackend: IBackend> {
    backend: TBackend,

    instance_table: HashMap<TBackend::RenderTargetId, Instance<TBackend>>,

    background_rendering_instance_table: HashMap<TBackend::RenderTargetId, Instance<TBackend>>,

    sync_semaphore_table: HashMap<TBackend::RenderTargetId, SyncSemaphores<TBackend>>,
}

impl Renderer<BackendVk> {
    pub fn new() -> Self {
        Self::new_with()
    }
}

impl<TBackend: IBackend> Renderer<TBackend> {
    pub fn new_with() -> Self {
        let backend: TBackend = TBackend::new();

        Self {
            backend,
            instance_table: HashMap::default(),
            background_rendering_instance_table: HashMap::default(),
            sync_semaphore_table: HashMap::default(),
        }
    }

    pub fn register_surface(
        &mut self,
        window_handle: WindowHandle,
        display_handle: DisplayHandle,
    ) -> Result<TBackend::RenderTargetId, ()> {
        let target_id = self
            .backend
            .register_surface(window_handle, display_handle)
            .unwrap();

        let background_pipeline_id = self
            .backend
            .create_pipeline(
                target_id,
                include_bytes!("../res/background.vs.spv"),
                include_bytes!("../res/background.fs.spv"),
            )
            .unwrap();

        let pipeline_id = self
            .backend
            .create_pipeline(
                target_id,
                include_bytes!("../res/character.vs.spv"),
                include_bytes!("../res/character.fs.spv"),
            )
            .unwrap();

        let vertex_buffer_id = self
            .backend
            .allocate_buffer(target_id, 64, BufferUsage::VertexBuffer)
            .unwrap();

        if let Ok(mut handle) = self.backend.map_buffer(target_id, vertex_buffer_id) {
            let data = bytemuck::cast_slice(&[-0.5f32, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5]);
            handle.write(0, data);
            self.backend
                .flush_buffer(target_id, vertex_buffer_id, 0 /*offset*/, 64);
        }

        let index_buffer_id = self
            .backend
            .allocate_buffer(target_id, 64, BufferUsage::IndexBuffer)
            .unwrap();
        if let Ok(mut handle) = self.backend.map_buffer(target_id, index_buffer_id) {
            let data = bytemuck::cast_slice(&[0u32, 1, 2, 0, 2, 3]);
            handle.write(0, data);
            // self.backend
            //     .flush_buffer(target_id, index_buffer_id, 0 /*offset*/, data.len());
            self.backend
                .flush_buffer(target_id, index_buffer_id, 0 /*offset*/, 64);
        }

        // 背景描画
        let background_constant_buffer_id = self
            .backend
            .allocate_buffer(target_id, 64, BufferUsage::UniformBuffer)
            .unwrap();
        if let Ok(mut handle) = self
            .backend
            .map_buffer(target_id, background_constant_buffer_id)
        {
            handle.write(
                0,
                bytemuck::cast_slice(&[BackgroundView {
                    texcoord_transform0: [1.0, 0.0, 0.0, 0.0],
                    texcoord_transform1: [0.0, 1.0, 0.0, 0.0],
                }]),
            );

            self.backend.flush_buffer(
                target_id,
                background_constant_buffer_id,
                0,
                std::mem::size_of::<BackgroundView>(),
            );
        }
        let background_descriptor_set = self
            .backend
            .allocate_descriptor_set(target_id, background_pipeline_id)
            .unwrap();
        self.backend.update_descriptors(&UpdateDescriptorsParams {
            render_target_id: target_id,
            descriptor_set_id: background_descriptor_set,
            buffer_info: vec![UpdateBufferInfo {
                id: background_constant_buffer_id,
                offset: 0,
                size: 32,
                usage: BufferDescriptorType::UniformBuffer,
            }],
        });

        // 文字ごとの情報
        let character_ssbo = self
            .backend
            .allocate_buffer(target_id, 256, BufferUsage::UnorderedAccessBuffer)
            .unwrap();
        if let Ok(mut handle) = self.backend.map_buffer(target_id, character_ssbo) {
            let data = bytemuck::cast_slice(&[
                CharacterData {
                    transform0: [0.2, 0.0, 0.0, 0.5],
                    transform1: [0.0, 0.2, 0.0, 0.5],
                    fore_ground_color: [1.0, 0.0, 1.0, 1.0],
                    uv_bl: [0.0, 0.0],
                    uv_tr: [0.0, 0.0],
                },
                CharacterData {
                    transform0: [0.2, 0.0, 0.0, -0.5],
                    transform1: [0.0, 0.2, 0.0, 0.5],
                    fore_ground_color: [1.0, 1.0, 1.0, 1.0],
                    uv_bl: [0.0, 0.0],
                    uv_tr: [0.0, 0.0],
                },
                CharacterData {
                    transform0: [0.2, 0.0, 0.0, -0.5],
                    transform1: [0.0, 0.2, 0.0, -0.5],
                    fore_ground_color: [0.0, 0.0, 1.0, 1.0],
                    uv_bl: [0.0, 0.0],
                    uv_tr: [0.0, 0.0],
                },
                CharacterData {
                    transform0: [0.2, 0.0, 0.0, 0.5],
                    transform1: [0.0, 0.2, 0.0, -0.5],
                    fore_ground_color: [1.0, 1.0, 0.0, 1.0],
                    uv_bl: [0.0, 0.0],
                    uv_tr: [0.0, 0.0],
                },
            ]);
            handle.write(0, data);

            self.backend
                .flush_buffer(target_id, character_ssbo, 0 /*offset*/, 64);
        }

        let descriptor_set_id = self
            .backend
            .allocate_descriptor_set(target_id, pipeline_id)
            .unwrap();
        self.backend.update_descriptors(&UpdateDescriptorsParams {
            render_target_id: target_id,
            descriptor_set_id,
            buffer_info: vec![UpdateBufferInfo {
                id: character_ssbo,
                offset: 0,
                size: std::mem::size_of::<CharacterData>() as usize * 4,
                usage: BufferDescriptorType::StorageBuffer,
            }],
        });

        // 背景描画
        // 頂点バッファーとインデックスバッファーは使い回す
        self.background_rendering_instance_table.insert(
            target_id,
            Instance {
                pipeline_id: background_pipeline_id,
                descriptor_set_id: Some(background_descriptor_set),
                vertex_buffer_id,
                index_buffer_id,
            },
        );

        // 文字の描画
        self.instance_table.insert(
            target_id,
            Instance {
                pipeline_id,
                descriptor_set_id: Some(descriptor_set_id),
                vertex_buffer_id,
                index_buffer_id,
            },
        );

        // 描画の同期用セマフォ
        self.sync_semaphore_table.insert(
            target_id,
            SyncSemaphores {
                acquire_next_image_semaphore_id: self.backend.create_semaphore(target_id).unwrap(),
                background_render_signal_semaphore_id: self
                    .backend
                    .create_semaphore(target_id)
                    .unwrap(),
                character_render_signal_semaphore_id: self
                    .backend
                    .create_semaphore(target_id)
                    .unwrap(),
            },
        );

        Ok(target_id)
    }

    pub fn render(&mut self) {
        let render_target_id = self.instance_table.keys().next().unwrap();
        let Some(instance) = self.instance_table.get(&render_target_id) else {
            return;
        };

        let Some(background_instance) = self
            .background_rendering_instance_table
            .get(&render_target_id)
        else {
            return;
        };

        let Some(semaphores) = self.sync_semaphore_table.get(&render_target_id) else {
            return;
        };

        // 次のフレームを要求
        // フレームが使用可能になってから描画コマンドが実行されるようにセマフォを指定して同期をとる
        let next_image_index = self
            .backend
            .acquire_next_frame(
                *render_target_id,
                semaphores.acquire_next_image_semaphore_id,
            )
            .unwrap();

        // 描画コマンドを実行
        // 開始と完了はセマフォを指定してフレームとの同期をとる

        // 背景の描画
        let background_render_params = RenderParams {
            render_target_id: *render_target_id,
            process_index: next_image_index,
            wait_semaphore_id: semaphores.acquire_next_image_semaphore_id,
            signal_semaphore_id: semaphores.background_render_signal_semaphore_id,
            pipelie_id: background_instance.pipeline_id,
            descriptor_set_id: background_instance.descriptor_set_id,
            vertex_buffer_id: background_instance.vertex_buffer_id,
            index_buffer_id: background_instance.index_buffer_id,
            index_count: 6,
            instance_count: 1,
        };
        self.backend.render(background_render_params);

        // 文字の描画
        let render_params = RenderParams {
            render_target_id: *render_target_id,
            process_index: next_image_index,
            wait_semaphore_id: semaphores.background_render_signal_semaphore_id,
            signal_semaphore_id: semaphores.character_render_signal_semaphore_id,
            pipelie_id: instance.pipeline_id,
            descriptor_set_id: instance.descriptor_set_id,
            vertex_buffer_id: instance.vertex_buffer_id,
            index_buffer_id: instance.index_buffer_id,
            index_count: 6,
            instance_count: 4,
        };
        self.backend.render(render_params);

        // 描画結果をウィンドウに表示する
        // 描画コマンドが完了してから実行されるようにセマフォで同期をとる
        self.backend.present(
            next_image_index,
            *render_target_id,
            semaphores.character_render_signal_semaphore_id,
        );
    }
}

impl<TBackend: IBackend> Drop for Renderer<TBackend> {
    fn drop(&mut self) {
        self.instance_table.retain(|key, value| {
            self.backend.destroy_buffer(*key, value.index_buffer_id);
            self.backend.destroy_buffer(*key, value.vertex_buffer_id);
            false
        });
    }
}
