use std::collections::HashMap;

use raw_window_handle::{DisplayHandle, WindowHandle};
use wgpu::RenderBundleDescriptor;

use crate::{
    backends::BackendVk,
    traits::{BufferUsage, IMapHandle, RenderParams},
    IBackend,
};

struct Instance<TBackend: IBackend> {
    pipeline_id: TBackend::PipelineId,
    vertex_buffer_id: TBackend::BufferId,
    index_buffer_id: TBackend::BufferId,
}

pub trait IRenderDecorator {
    fn render(&self);
}

struct RenderDecoratorConcrete;
impl IRenderDecorator for RenderDecoratorConcrete {
    fn render(&self) {}
}

struct RenderDecoratorAdapter<T: IRenderDecorator, U: IRenderDecorator> {
    decorator: T,
    internal_impl: U,
}
impl<T: IRenderDecorator, U: IRenderDecorator> IRenderDecorator for RenderDecoratorAdapter<T, U> {
    fn render(&self) {
        self.internal_impl.render();
        self.decorator.render();
    }
}

struct RendererBuilder<T: IRenderDecorator> {
    decorator: T,
}

impl RendererBuilder<RenderDecoratorConcrete> {
    fn new() -> Self {
        Self {
            decorator: RenderDecoratorConcrete,
        }
    }
}

impl<T: IRenderDecorator> RendererBuilder<T> {
    pub fn build(self) -> Renderer<BackendVk, T> {
        Renderer::new(self.decorator)
    }

    pub fn decorate<U: IRenderDecorator>(
        self,
        decorator: U,
    ) -> RendererBuilder<RenderDecoratorAdapter<U, T>> {
        let adapter = RenderDecoratorAdapter {
            decorator,
            internal_impl: self.decorator,
        };
        RendererBuilder { decorator: adapter }
    }
}

pub struct Renderer<TBackend: IBackend, TDecorator: IRenderDecorator> {
    backend: TBackend,

    instance_table: HashMap<TBackend::RenderTargetId, Instance<TBackend>>,

    decorator: TDecorator,
}

impl<TDecorator: IRenderDecorator> Renderer<BackendVk, TDecorator> {
    pub fn new(decorator: TDecorator) -> Self {
        Self::new_with(decorator)
    }
}

impl<TBackend: IBackend, TDecorator: IRenderDecorator> Renderer<TBackend, TDecorator> {
    pub fn builder() -> RendererBuilder<RenderDecoratorConcrete> {
        RendererBuilder::new()
    }

    fn new_with(decorator: TDecorator) -> Renderer<TBackend, TDecorator> {
        let backend: TBackend = TBackend::new();

        Self {
            backend,
            instance_table: HashMap::default(),
            decorator,
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

        let pipeline_id = self.backend.create_pipeline(target_id, &[], &[]).unwrap();

        let vertex_buffer_id = self
            .backend
            .allocate_buffer(target_id, 64, BufferUsage::VertexBuffer)
            .unwrap();

        if let Ok(mut handle) = self.backend.map_buffer(target_id, vertex_buffer_id) {
            handle.write(0 /*offset*/, &[]);
        }

        let index_buffer_id = self
            .backend
            .allocate_buffer(target_id, 64, BufferUsage::IndexBuffer)
            .unwrap();

        self.instance_table.insert(
            target_id,
            Instance {
                pipeline_id,
                vertex_buffer_id,
                index_buffer_id,
            },
        );

        Ok(target_id)
    }

    pub fn render(&self) {
        let render_target_id = self.instance_table.keys().next().unwrap();
        let Some(instance) = self.instance_table.get(&render_target_id) else {
            return;
        };

        let render_params = RenderParams {
            render_target_id: *render_target_id,
            pipelie_id: instance.pipeline_id,
            vertex_buffer_id: instance.vertex_buffer_id,
            index_buffer_id: instance.index_buffer_id,
            index_count: 3,
            instance_count: 1,
        };
        self.backend.render(render_params);
    }
}

impl<TBackend: IBackend, T: IRenderDecorator> Drop for Renderer<TBackend, T> {
    fn drop(&mut self) {
        self.instance_table.retain(|key, value| {
            self.backend.destroy_buffer(*key, value.index_buffer_id);
            self.backend.destroy_buffer(*key, value.vertex_buffer_id);
            false
        });
    }
}
