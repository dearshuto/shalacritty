use std::{borrow::Cow, collections::HashMap, marker::PhantomData};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::window::WindowId;

use crate::gfx::content_plotter::Diff;

#[repr(C)]
#[derive(Debug, Pod, Copy, Clone, Zeroable)]
struct CharacterData {
    transform0: [f32; 4],
    transform1: [f32; 4],
    fore_ground_color: [f32; 4],
    uv_bl: [f32; 2],
    uv_tr: [f32; 2],
}

pub struct TextRenderer<'a> {
    pipelie_table: HashMap<WindowId, wgpu::RenderPipeline>,
    vertex_buffer_table: HashMap<WindowId, wgpu::Buffer>,
    index_buffer_table: HashMap<WindowId, wgpu::Buffer>,
    bind_group_table: HashMap<WindowId, wgpu::BindGroup>,
    character_storage_block_table: HashMap<WindowId, wgpu::Buffer>,
    sampler_table: HashMap<WindowId, wgpu::Sampler>,
    glyph_texture: Option<wgpu::Texture>,

    character_count: u32,

    _phantom_data: PhantomData<&'a ()>,
}

impl<'a> TextRenderer<'a> {
    pub fn new() -> Self {
        Self {
            pipelie_table: HashMap::default(),
            vertex_buffer_table: HashMap::default(),
            index_buffer_table: HashMap::default(),
            bind_group_table: HashMap::default(),
            character_storage_block_table: HashMap::default(),
            sampler_table: HashMap::default(),
            glyph_texture: None,
            character_count: 0,
            _phantom_data: Default::default(),
        }
    }

    pub async fn register(
        &mut self,
        id: WindowId,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
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

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<f32>() * 2) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        }];

        let vertex_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../char_rect.vs.wgsl"))),
        });

        let pixel_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../char_rect.fs.wgsl"))),
        });
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
        let vertrex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

        // 文字ごとの情報
        let character_storage_block = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<CharacterData>() as u64 * 32 * 1024,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
                width: 4096,
                height: 4096,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::R8Unorm],
        });

        // リソースたちのバインド設定
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: character_storage_block.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.create_view(
                        &wgpu::TextureViewDescriptor {
                            label: None,
                            format: Some(wgpu::TextureFormat::R8Unorm),
                            dimension: Some(wgpu::TextureViewDimension::D2),
                            aspect: wgpu::TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: 0,
                            array_layer_count: None,
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.pipelie_table.insert(id, render_pipeline);
        self.vertex_buffer_table.insert(id, vertrex_buffer);
        self.index_buffer_table.insert(id, index_buffer);
        self.character_storage_block_table
            .insert(id, character_storage_block);
        self.bind_group_table.insert(id, bind_group);
        self.sampler_table.insert(id, sampler);
        self.glyph_texture = Some(texture);
    }

    pub fn update(&mut self, queue: &wgpu::Queue, id: WindowId, diff: &Diff) {
        let buffer = self.character_storage_block_table.get(&id).unwrap();

        // 文字数
        self.character_count = diff.item_count() as u32;

        let data = diff
            .character_info_array()
            .iter()
            .map(|info| {
                let t = info.transform;
                (
                    info.index,
                    CharacterData {
                        transform0: [t[0], t[1], t[2], 0.0],
                        transform1: [t[3], t[4], t[5], 0.0],
                        fore_ground_color: info.fore_ground_color,
                        uv_bl: [info.uv0[0], info.uv0[1]],
                        uv_tr: [info.uv1[0], info.uv1[1]],
                    },
                )
            })
            .collect::<Vec<(usize, CharacterData)>>();

        if !data.is_empty() {
            for (index, data) in data {
                let offset = index * std::mem::size_of::<CharacterData>();
                let binary = bytemuck::bytes_of(&data);
                queue.write_buffer(buffer, offset as u64, binary);
            }
        }

        let texture = self.glyph_texture.as_ref().unwrap();
        for texture_patch in diff.glyph_texture_patches() {
            let image_copy = wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: texture_patch.offset_x(),
                    y: texture_patch.offset_y(),
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            };
            queue.write_texture(
                image_copy,
                texture_patch.pixels(),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(texture_patch.width()),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: texture_patch.width(),
                    height: texture_patch.height(),
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub fn render(&'a self, id: WindowId, mut render_pass: wgpu::RenderPass<'a>) {
        let Some(pipeline) = self.pipelie_table.get(&id) else {
            return;
        };

        let Some(vertex_buffer) = self.vertex_buffer_table.get(&id) else {
            return;
        };

        let Some(index_buffer) = self.index_buffer_table.get(&id) else {
            return;
        };

        let Some(bind_group) = self.bind_group_table.get(&id) else {
            return;
        };

        render_pass.set_pipeline(pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw_indexed(0..6, 0, 0..1);
        render_pass.draw_indexed(0..6, 0, 0..self.character_count);
    }
}
