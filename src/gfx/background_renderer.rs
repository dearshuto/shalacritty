use wgpu::{include_spirv_raw, util::DeviceExt};
use winit::window::WindowId;

struct ConstantBufferData {}

struct Instance {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    constant_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    #[allow(dead_code)]
    texture: wgpu::Texture,
}

pub struct BackgroundRenderer<'a> {
    instance: Option<Instance>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> BackgroundRenderer<'a> {
    pub fn new() -> Self {
        Self {
            instance: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn register(&mut self, _id: WindowId, device: &wgpu::Device, format: wgpu::TextureFormat) {
        let vertex_shader_module_spirv = include_spirv_raw!("background.vs.spv");
        let vertex_shader_module =
            unsafe { device.create_shader_module_spirv(&vertex_shader_module_spirv) };

        let pixel_shader_module_spirv = include_spirv_raw!("background.fs.spv");
        let pixel_shader_module =
            unsafe { device.create_shader_module_spirv(&pixel_shader_module_spirv) };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
            //     wgpu::BindGroupLayoutEntry {
            //     binding: 0,
            //     visibility: wgpu::ShaderStages::VERTEX,
            //     ty: wgpu::BindingType::Buffer {
            //         ty: wgpu::BufferBindingType::Uniform,
            //         has_dynamic_offset: false,
            //         min_binding_size: None,
            //     },
            //     count: None,
            // }
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

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
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview: None,
        });

        // 頂点バッファー
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[-1.0f32, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0]),
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
            size: std::mem::size_of::<ConstantBufferData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[],
            // entries: &[wgpu::BindGroupEntry {
            //     binding: 0,
            //     resource: constant_buffer.as_entire_binding(),
            // }],
        });

        // グリフを矩形に貼るときのサンプラー
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        // 文字テクスチャ。一番複雑なところなので最終的には外部で管理する。
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8Uint],
        });

        self.instance = Some(Instance {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            bind_group,
            constant_buffer,
            sampler,
            texture,
        });
    }

    pub fn resize(&self, _id: WindowId, _width: u32, _height: u32) {}

    pub fn update(&self, _id: WindowId, _device: &wgpu::Device, _queue: &wgpu::Queue) {
        //
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
