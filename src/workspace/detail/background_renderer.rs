use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    rc::Rc,
};

use image::{
    codecs::{jpeg::JpegDecoder, png::PngDecoder},
    DynamicImage, GenericImageView,
};
use uuid::Uuid;
use wgpu::util::DeviceExt;

use crate::gfx::{IRenderPlugin, UpdateParams};

#[derive(bytemuck::NoUninit, Clone, Copy, Debug)]
#[repr(C)]
struct ConstantBufferData {
    image_tansform0: [f32; 4],
    image_tansform1: [f32; 4],
}

#[derive(bytemuck::NoUninit, Clone, Copy, Debug)]
#[repr(C)]
struct MaterialData {
    alpha_enhance: f32,
}

struct Instance {
    // 背景描画パイプライン
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,

    index_buffer: wgpu::Buffer,

    // 画像の UV
    constant_buffer: wgpu::Buffer,

    material_constant_buffer: wgpu::Buffer,

    sampler: wgpu::Sampler,

    // テクスチャー
    texture_table: HashMap<BackgroundId, wgpu::Texture>,

    // テクスチャーサイズ
    texture_size_table: HashMap<BackgroundId, (u32, u32)>,

    enhance_value_table: HashMap<BackgroundId, f32>,

    resize_dirty_flag: bool,

    // バインドするリソース
    bind_group_table: HashMap<BackgroundId, wgpu::BindGroup>,
}

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub struct BackgroundId {
    id: Uuid,
}

pub struct BackgroundRenderer<'a> {
    instance: Option<Instance>,

    active_background_id: Option<BackgroundId>,

    path_table: HashMap<BackgroundId, PathBuf>,

    window_size: (u32, u32),

    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> BackgroundRenderer<'a> {
    pub fn new() -> Self {
        Self {
            instance: None,
            active_background_id: None,
            path_table: HashMap::default(),
            window_size: (640, 480),
            _marker: std::marker::PhantomData,
        }
    }

    // 8 Bit Unorm sRGB 固定
    pub fn register<TPath>(&mut self, path: TPath) -> BackgroundId
    where
        TPath: AsRef<Path>,
    {
        let id = BackgroundId { id: Uuid::new_v4() };
        self.path_table.insert(id, path.as_ref().to_path_buf());
        self.active_background_id = Some(id);
        id
    }

    pub fn activate(&mut self, id: BackgroundId, enhance: f32) -> bool {
        // 変化がなければなにもしない
        if self.active_background_id.is_some() && self.active_background_id.unwrap() == id {
            return false;
        }

        self.active_background_id = Some(id);

        // 画像を切り替えるとウィンドウにフィットさせるための領域も変更になるのでダーティにする
        let Some(instance) = &mut self.instance else {
            return true;
        };

        instance.resize_dirty_flag = true;
        instance.enhance_value_table.insert(id, enhance);
        true
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
                contents: bytemuck::bytes_of(&MaterialData { alpha_enhance: 1.0 }),
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
            texture_size_table: HashMap::default(),
            enhance_value_table: HashMap::default(),
            resize_dirty_flag: true,
            bind_group_table: HashMap::default(),
        }
    }

    fn update_image(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(instance) = &mut self.instance else {
            return;
        };
        for (id, path) in &self.path_table {
            if instance.texture_table.contains_key(id) {
                continue;
            }

            let Some(image) = Self::load_image(path) else {
                continue;
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
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
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
            });

            let mut data = Vec::default();
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let pixel = image.get_pixel(x, y);
                    data.push(pixel[0]);
                    data.push(pixel[1]);
                    data.push(pixel[2]);
                    data.push(pixel[3]);
                }
            }
            queue.write_texture(
                texture.as_image_copy(),
                &data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * image.width()),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
            );

            // メンバに追加
            instance.texture_table.insert(*id, texture);
            instance
                .texture_size_table
                .insert(*id, (image.width(), image.height()));
        }
    }

    fn update_bind_group(&mut self, device: &wgpu::Device) {
        let Some(instance) = &mut self.instance else {
            return;
        };

        for (id, texture) in &instance.texture_table {
            // すでに作ってあたらなにもしない
            if instance.bind_group_table.contains_key(id) {
                continue;
            }

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
                        resource: wgpu::BindingResource::TextureView(&texture.create_view(
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

            instance.bind_group_table.insert(*id, bind_group);
        }
    }

    fn update_size(&mut self, queue: &wgpu::Queue) {
        let Some(instance) = &mut self.instance else {
            return;
        };

        if !instance.resize_dirty_flag {
            return;
        } else {
            instance.resize_dirty_flag = false;
        }

        let Some(id) = self.active_background_id else {
            return;
        };
        let Some((image_width, image_height)) = instance.texture_size_table.get(&id) else {
            return;
        };

        let (width, height) = self.window_size;

        let width = width as f32;
        let height = height as f32;
        let image_width = *image_width as f32;
        let image_height = *image_height as f32;

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

        if let Some(enhance) = instance.enhance_value_table.get(&id) {
            let data = MaterialData {
                alpha_enhance: *enhance,
            };
            let data = bytemuck::bytes_of(&data);
            queue.write_buffer(&instance.material_constant_buffer, 0, data);
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

    fn load_image<TPath>(path: TPath) -> Option<DynamicImage>
    where
        TPath: AsRef<Path>,
    {
        if !path.as_ref().is_file() {
            return None;
        }

        let mut reader = File::open(path.as_ref()).unwrap();
        let path = Path::new(path.as_ref());

        if path.extension().unwrap() == "png" {
            let decoder = PngDecoder::new(&mut reader).unwrap();
            Some(DynamicImage::from_decoder(decoder).unwrap())
        } else {
            let decoder = JpegDecoder::new(&mut reader).unwrap();
            // let decoder = JpegDecoder::new(&mut reader).unwrap();
            Some(DynamicImage::from_decoder(decoder).unwrap())
        }
    }
}

impl<'a> IRenderPlugin for Rc<RefCell<BackgroundRenderer<'a>>> {
    type UserData = ();

    fn update(&mut self, update_params: &UpdateParams<()>) {
        let device = update_params.device();
        let queue = update_params.queue();

        let mut renderer = self.borrow_mut();
        if renderer.instance.is_none() {
            renderer.instance = Some(BackgroundRenderer::create_instance(device));
        }

        let _ = update_params.user_data();

        // 画像更新
        renderer.update_image(device, queue);

        // BindGroup の更新
        renderer.update_bind_group(device);

        // サイズの変更を反映
        renderer.update_size(queue);
    }

    fn register(&mut self, _instance: &wgpu::Instance) {}

    fn resize(&mut self, width: u32, height: u32) {
        self.borrow_mut().window_size = (width, height);

        if let Some(instance) = &mut self.borrow_mut().instance {
            instance.resize_dirty_flag = true;
        };
    }

    fn render(
        &self,
        render_target: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
    ) {
        let Some(id) = self.borrow().active_background_id else {
            return;
        };

        let binding = self.borrow();
        let Some(instance) = &binding.instance else {
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
            binding.window_size.0 as f32,
            binding.window_size.1 as f32,
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
