use std::{borrow::Cow, collections::HashMap};

use wgpu::util::DeviceExt;
use winit::window::WindowId;

struct Instance {
    color_target: wgpu::Texture,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,

    #[allow(dead_code)]
    sampler: wgpu::Sampler,

    constant_buffer: wgpu::Buffer,
}

pub struct ScanBufferRenderer<'a> {
    // スキャンバッファーにコピーするインスタンス
    instance_table: HashMap<WindowId, Instance>,

    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> ScanBufferRenderer<'a> {
    pub fn new() -> Self {
        Self {
            instance_table: HashMap::default(),
            _marker: Default::default(),
        }
    }

    pub fn register(
        &mut self,
        id: WindowId,
        device: &wgpu::Device,
        swapchain_format: wgpu::TextureFormat,
    ) {
        let color_target = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 2048,
                height: 2048,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: swapchain_format,
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[swapchain_format],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[1.0f32, 1.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                        "copy_scan_buffer.vs.wgsl"
                    ))),
                }),
                entry_point: "main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                        "copy_scan_buffer.fs.wgsl"
                    ))),
                }),
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &color_target.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.instance_table.insert(
            id,
            Instance {
                color_target,
                render_pipeline,
                bind_group,
                sampler,
                constant_buffer: buffer,
            },
        );
    }

    pub fn resize(&self, id: WindowId, queue: &wgpu::Queue, width: u32, height: u32) {
        let (scale_x, scale_y) = (width as f32 / 2048.0, height as f32 / 2048.0);
        let instance = self.instance_table.get(&id).unwrap();
        queue.write_buffer(
            &instance.constant_buffer,
            0,
            bytemuck::cast_slice(&[scale_x, scale_y]),
        );
    }

    pub fn render(&'a self, id: WindowId, mut render_pass: wgpu::RenderPass<'a>) {
        let instance = self.instance_table.get(&id).unwrap();
        render_pass.set_pipeline(&instance.render_pipeline);
        render_pass.set_bind_group(0, &instance.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }

    pub fn create_view(&self, id: WindowId) -> wgpu::TextureView {
        let color_target = &self.instance_table.get(&id).unwrap().color_target;
        color_target.create_view(&wgpu::TextureViewDescriptor::default())
    }
}
