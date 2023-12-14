use std::num::NonZeroU64;

use wgpu::{include_spirv_raw, util::DeviceExt};
use winit::window::WindowId;

use crate::gfx::content_plotter::Diff;

struct View {
    #[allow(dead_code)]
    transform: [f32; 64],
}

struct Instance {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    constant_buffer: wgpu::Buffer,
}

pub struct CursorRenderer<'a> {
    instance: Option<Instance>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> CursorRenderer<'a> {
    pub fn new() -> Self {
        Self {
            instance: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn register(
        &mut self,
        _id: WindowId,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<f32>() as u64 * 8),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_shader_module_spirv = include_spirv_raw!("rect.vs.spv");
        let vertex_shader_module =
            unsafe { device.create_shader_module_spirv(&vertex_shader_module_spirv) };

        let pixel_shader_module_spirv = include_spirv_raw!("rect.fs.spv");
        let pixel_shader_module =
            unsafe { device.create_shader_module_spirv(&pixel_shader_module_spirv) };

        // 頂点アトリビュート
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<f32>() * 2) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        }];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader_module,
                entry_point: "main",
                buffers: &vertex_buffers,
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &pixel_shader_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
        });

        // 頂点バッファー
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[0.0f32, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // インデックスバッファー
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });

        // 定数バッファー
        let constant_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            size: std::mem::size_of::<View>() as u64,
            mapped_at_creation: false,
        });

        // リソースたちのバインド設定
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: constant_buffer.as_entire_binding(),
            }],
        });

        let instance = Instance {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            bind_group,
            constant_buffer,
        };
        self.instance = Some(instance);
    }

    pub fn update(&mut self, _id: WindowId, diff: &Diff, queue: &wgpu::Queue) {
        let Some(cursor) = diff.cursor() else {
            return;
        };

        let Some(instance) = &self.instance else {
            return;
        };

        // [0, 1] に正規化
        let (x, y) = {
            (
                cursor.point.column.0 as f32 / 64.0,
                cursor.point.line.0 as f32 / 64.0,
            )
        };

        // [0, 1] -> [-1, 1]
        let (screen_x, screen_y) = (2.0 * x - 1.0, 2.0 * y - 1.0);

        let data = [
            1.0f32 / 256.0, // それっぽいサイズを目あわせ
            0.0,
            screen_x,
            0.0,
            0.0,
            1.0 / 24.0, // それっぽいサイズを目あわせ
            screen_y,
            0.0,
        ];
        queue.write_buffer(
            &instance.constant_buffer,
            0, /*offset*/
            bytemuck::cast_slice(&data),
        );
    }

    pub fn render(&'a self, _id: WindowId, mut render_pass: wgpu::RenderPass<'a>) {
        let Some(instance) = &self.instance else {
            return;
        };

        render_pass.set_pipeline(&instance.render_pipeline);
        render_pass.set_vertex_buffer(0, instance.vertex_buffer.slice(..));
        render_pass.set_index_buffer(instance.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, &instance.bind_group, &[]);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}
