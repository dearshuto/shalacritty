use std::{borrow::Cow, collections::HashMap, u8};

use wgpu::util::DeviceExt;

use crate::gfx::{IRenderPlugin, UpdateParams};

use super::{ImageCache, ImageId};

const INIT_IMAGE_ALPHA: f32 = 0.3;

// 頂点シェーダーに渡す定数バッファーの型定義
#[derive(bytemuck::NoUninit, Clone, Copy, Debug)]
#[repr(C)]
struct ConstantBufferData {
    image_tansform0: [f32; 4],
    image_tansform1: [f32; 4],
}

// ピクセルシェーダーに渡す定数バッファーの型定義
#[derive(bytemuck::NoUninit, Clone, Copy, Debug)]
#[repr(C)]
struct MaterialData {
    alpha_enhance: f32,
}

#[allow(dead_code)]
struct Instance {
    // 背景描画パイプライン
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,

    index_buffer: wgpu::Buffer,

    // 定数バッファー
    constant_buffer: wgpu::Buffer,

    material_constant_buffer: wgpu::Buffer,

    sampler: wgpu::Sampler,

    // テクスチャーの元画像の世代
    image_generation_table: HashMap<ImageId, u64>,

    // テクスチャー
    texture_table: HashMap<ImageId, wgpu::Texture>,

    // 画像を画面全体にフィットさせるための UV 変換用の行列
    uv_transform_table: HashMap<ImageId, nalgebra::Matrix4<f32>>,

    // 画像にかけあわせる色
    enhance_value_table: HashMap<ImageId, f32>,

    // リサイズによるダーティフラグ
    resize_dirty_table: HashMap<ImageId, bool>,

    // バインドするリソース
    bind_group_table: HashMap<ImageId, wgpu::BindGroup>,

    active_image_id_cache: Option<ImageId>,

    window_size: (u32, u32),

    image_alpha_chache: f32,
}

pub trait IBackgroundRendererContext {
    fn active_id(&self) -> Option<ImageId>;

    // [0, 1]
    fn active_image_alpha(&self) -> f32;

    fn image_cache(&self) -> &ImageCache;

    fn window_size(&self) -> (u32, u32);
}

pub struct BackgroundRenderer<T> {
    instance: Option<Instance>,

    active_id: Option<ImageId>,

    _marker: std::marker::PhantomData<T>,
}

impl<T> BackgroundRenderer<T> {
    pub fn new() -> Self {
        Self {
            instance: None,
            active_id: None,
            _marker: std::marker::PhantomData,
        }
    }

    fn create_instance(device: &wgpu::Device) -> Instance {
        let vertex_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../../gfx/detail/background.vs.wgsl"
            ))),
        });

        let pixel_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../../gfx/detail/background.fs.wgsl"
            ))),
        });

        let bind_group_layout = Self::create_bind_group_layout(device);

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
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &pixel_shader_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::DstAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::DstAlpha,
                            operation: wgpu::BlendOperation::Min,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::all(),
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
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

        // UV 用の定数バッファー
        let constant_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<ConstantBufferData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let constant_buffer_material =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::bytes_of(&MaterialData {
                    alpha_enhance: INIT_IMAGE_ALPHA,
                }),
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

        Instance {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            constant_buffer,
            material_constant_buffer: constant_buffer_material,
            sampler,
            texture_table: HashMap::default(),
            image_generation_table: HashMap::default(),
            uv_transform_table: HashMap::default(),
            enhance_value_table: HashMap::default(),
            resize_dirty_table: HashMap::default(),
            bind_group_table: HashMap::default(),
            active_image_id_cache: None,
            window_size: (640, 480),
            image_alpha_chache: INIT_IMAGE_ALPHA,
        }
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }
}

impl<T: IBackgroundRendererContext> IRenderPlugin for BackgroundRenderer<T> {
    type UserData = T;

    fn update(&mut self, update_params: &UpdateParams<'_, Self::UserData>) {
        // 初回はインスタンスを生成してないので作成する
        if self.instance.is_none() {
            let device = update_params.device();
            self.instance = Some(BackgroundRenderer::<T>::create_instance(device));
        }

        let is_image_changed = !(self.active_id.is_some()
            && update_params.user_data().active_id().is_some()
            && self.active_id.unwrap() == update_params.user_data().active_id().unwrap());
        self.active_id = update_params.user_data().active_id();

        let instance = self.instance.as_mut().unwrap();
        let user_data = update_params.user_data();
        let Some(active_image_id) = user_data.active_id() else {
            return;
        };

        let image_cache = user_data.image_cache();
        let queue = update_params.queue();
        if is_image_changed || instance.window_size != user_data.window_size() {
            image_cache.operate_image(active_image_id, |image| {
                let Some(image) = image else {
                    return;
                };

                // テクスチャーを画面にフィットさせるための UV 変換を算出
                let (width, height) = user_data.window_size();
                let width = width as f32;
                let height = height as f32;
                let image_width = image.width() as f32;
                let image_height = image.height() as f32;

                // 画像の UV 変換
                let scale_x = width / image_width;
                let scale_y = height / image_height;

                // (1, 1) より外を参照してたらフィットするよう補正
                // [0, 1] だったら補正は不要なので 1 で抑えておく
                let factor = scale_x.max(scale_y).max(1.0);
                let x = scale_x / factor;
                let y = scale_y / factor;

                // 画像の中心とターミナルの中心が一致するように並行移動
                let t_x = 0.5 * (image_width - width).max(0.0) / width;
                let t_y = 0.5 * (image_height - height).max(0.0) / height;
                let constant_buffer_data = ConstantBufferData {
                    image_tansform0: [x, 0.0, x * t_x, 0.0],
                    image_tansform1: [0.0, y, y * t_y, 0.0],
                };
                let constant_buffer_data = bytemuck::bytes_of(&constant_buffer_data);
                queue.write_buffer(&instance.constant_buffer, 0, constant_buffer_data);
                instance.window_size = user_data.window_size();
            });
        }

        // ピクセルシェーダー用の定数バッファー
        // 背景画像のアルファが変更されていたら更新する
        if instance.image_alpha_chache != user_data.active_image_alpha() {
            queue.write_buffer(
                &instance.material_constant_buffer,
                0,
                bytemuck::bytes_of(&MaterialData {
                    alpha_enhance: update_params.user_data().active_image_alpha(),
                }),
            );
            instance.image_alpha_chache = user_data.active_image_alpha();
        }

        // 画像データの最新の世代を取得
        let image_cache = user_data.image_cache();
        let current_generation = image_cache.get_generation(active_image_id);

        // キャッシュしてある世代の方が新しければなにもしない
        let cached_generation = instance
            .image_generation_table
            .insert(active_image_id, current_generation);
        if cached_generation.is_some() && current_generation <= cached_generation.unwrap() {
            return;
        }

        // 世代が進んでいたのでテクスチャーを作り直す
        let device = update_params.device();
        let queue = update_params.queue();
        image_cache.operate_image(active_image_id, |image| {
            let Some(image) = image else {
                return;
            };

            let raw_bytes = image.as_bytes();
            let raw_bytes = {
                let mut result = Vec::new();
                for index in 0..(raw_bytes.len() / 3) {
                    result.push(raw_bytes[3 * index]);
                    result.push(raw_bytes[3 * index + 1]);
                    result.push(raw_bytes[3 * index + 2]);
                    result.push(u8::MAX);
                }
                result
            };
            let new_texture = device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
                },
                wgpu::util::TextureDataOrder::default(),
                &raw_bytes,
            );

            let bind_group_layout = Self::create_bind_group_layout(device);
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: instance.constant_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&new_texture.create_view(
                            &wgpu::TextureViewDescriptor {
                                label: None,
                                format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
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
                        resource: wgpu::BindingResource::Sampler(&instance.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: instance.material_constant_buffer.as_entire_binding(),
                    },
                ],
            });
            instance
                .bind_group_table
                .insert(active_image_id, bind_group);
            instance.texture_table.insert(active_image_id, new_texture);
        });
    }

    fn register(&mut self, _instance: &wgpu::Instance) {}

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn render(
        &self,
        render_target: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
    ) {
        let Some(id) = self.active_id else {
            return;
        };

        let Some(instance) = &self.instance else {
            return;
        };

        let Some(bind_group) = instance.bind_group_table.get(&id) else {
            return;
        };

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_viewport(
            0.0,
            0.0,
            instance.window_size.0 as f32,
            instance.window_size.1 as f32,
            0.0,
            1.0,
        );

        render_pass.set_pipeline(&instance.render_pipeline);
        render_pass.set_vertex_buffer(0, instance.vertex_buffer.slice(..));
        render_pass.set_index_buffer(instance.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}
