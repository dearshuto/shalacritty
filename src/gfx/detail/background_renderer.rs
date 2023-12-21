use std::{
    fs::File,
    path::{Path, PathBuf},
};

use image::{
    codecs::{jpeg::JpegDecoder, png::PngDecoder},
    DynamicImage, GenericImageView,
};
use wgpu::{include_spirv_raw, util::DeviceExt};
use winit::window::WindowId;

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
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group_layout_cache: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    constant_buffer: wgpu::Buffer,
    material_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    #[allow(dead_code)]
    texture: wgpu::Texture,
    width: u32,
    height: u32,
}

pub struct BackgroundRenderer<'a> {
    instance: Option<Instance>,
    // 背景に表示している画像
    texture_path_cache: Option<PathBuf>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> BackgroundRenderer<'a> {
    pub fn new() -> Self {
        Self {
            instance: None,
            texture_path_cache: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn register(
        &mut self,
        _id: WindowId,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) {
        let instance =
            create_instance::<PathBuf>(device, queue, format, CreateInstanceParams::New, 1.0);
        self.instance = Some(instance);
    }

    pub fn resize(&self, _id: WindowId, queue: &wgpu::Queue, width: u32, height: u32) {
        let Some(instance) = &self.instance else {
            return;
        };

        let width = width as f32;
        let height = height as f32;
        let image_width = instance.width as f32;
        let image_height = instance.height as f32;

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
        queue.write_buffer(
            &self.instance.as_ref().unwrap().constant_buffer,
            0,
            constant_buffer_data,
        );
    }

    pub fn update<TPath>(
        &mut self,
        _id: WindowId,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_format: wgpu::TextureFormat,
        image_path: TPath,
        alpha_enhance: f32,
    ) where
        TPath: AsRef<Path>,
    {
        if self.texture_path_cache.is_some() {
            return;
        } else {
            self.texture_path_cache = Some(Path::new(".").to_path_buf());
        }

        let mut instance = None;
        std::mem::swap(&mut instance, &mut self.instance);

        let instance = create_instance(
            device,
            queue,
            texture_format,
            CreateInstanceParams::WithCache(instance.unwrap(), image_path),
            alpha_enhance,
        );
        queue.write_buffer(
            &instance.material_buffer,
            0,
            bytemuck::bytes_of(&MaterialData { alpha_enhance }),
        );
        self.instance = Some(instance);
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

enum CreateInstanceParams<TPath>
where
    TPath: AsRef<Path>,
{
    New,
    WithCache(Instance, TPath /*背景画像のパス*/),
}

fn create_instance<TPath>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    params: CreateInstanceParams<TPath>,
    alpha_enhance: f32,
) -> Instance
where
    TPath: AsRef<Path>,
{
    match params {
        CreateInstanceParams::New => {
            let vertex_shader_module_spirv = include_spirv_raw!("background.vs.spv");
            let vertex_shader_module =
                unsafe { device.create_shader_module_spirv(&vertex_shader_module_spirv) };

            let pixel_shader_module_spirv = include_spirv_raw!("background.fs.spv");
            let pixel_shader_module =
                unsafe { device.create_shader_module_spirv(&pixel_shader_module_spirv) };

            let bind_group_layout =
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
                            alpha: wgpu::BlendComponent::REPLACE,
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

            let constant_buffer_material =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    contents: bytemuck::bytes_of(&MaterialData { alpha_enhance }),
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

            let (bind_group, texture, sampler) = create_bind_group(
                device,
                &bind_group_layout,
                &constant_buffer,
                &constant_buffer_material,
                sampler,
                1,
                1,
            );

            Instance {
                render_pipeline,
                vertex_buffer,
                index_buffer,
                bind_group_layout_cache: bind_group_layout,
                bind_group,
                constant_buffer,
                material_buffer: constant_buffer_material,
                sampler,
                texture,
                width: 1,
                height: 1,
            }
        }
        CreateInstanceParams::WithCache(instance, path) => {
            // ファイルが見つからなかったらなにもしない
            if !path.as_ref().is_file() {
                return instance;
            }

            let mut reader = File::open(path.as_ref()).unwrap();
            let path = Path::new(path.as_ref());

            let image = if path.extension().unwrap() == "png" {
                let decoder = PngDecoder::new(&mut reader).unwrap();
                DynamicImage::from_decoder(decoder).unwrap()
            } else {
                let decoder = JpegDecoder::new(&mut reader).unwrap();
                // let decoder = JpegDecoder::new(&mut reader).unwrap();
                DynamicImage::from_decoder(decoder).unwrap()
            };
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

            let (bind_group, texture, sampler) = create_bind_group(
                device,
                &instance.bind_group_layout_cache,
                &instance.constant_buffer,
                &instance.material_buffer,
                instance.sampler,
                image.width(),
                image.height(),
            );

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

            Instance {
                render_pipeline: instance.render_pipeline,
                vertex_buffer: instance.vertex_buffer,
                index_buffer: instance.index_buffer,
                bind_group_layout_cache: instance.bind_group_layout_cache,
                bind_group,
                constant_buffer: instance.constant_buffer,
                material_buffer: instance.material_buffer,
                sampler,
                texture,
                width: image.width(),
                height: image.height(),
            }
        }
    }
}

// テクスチャーサイズを指定してバインドグループを作成します。
fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    constant_buffer: &wgpu::Buffer,
    constant_buffer_material: &wgpu::Buffer,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
) -> (wgpu::BindGroup, wgpu::Texture, wgpu::Sampler) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: constant_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture.create_view(
                    &wgpu::TextureViewDescriptor {
                        label: None,
                        format: Some(wgpu::TextureFormat::Rgba8Unorm),
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
            wgpu::BindGroupEntry {
                binding: 3,
                resource: constant_buffer_material.as_entire_binding(),
            },
        ],
    });

    (bind_group, texture, sampler)
}
