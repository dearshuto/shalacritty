use std::borrow::Cow;

use wgpu::util::DeviceExt;
use winit::window::WindowId;

use crate::{app::WindowSizeChangedEventArgs, gfx::GlyphTexturePatch, Config};

use super::ImageLoadedEventArgs;

const INIT_IMAGE_ALPHA: f32 = 0.3;

pub struct CharacterData {
    pub transform0: [f32; 4],
    pub transform1: [f32; 4],
    pub fore_ground_color: [f32; 4],
    pub uv_bl: [f32; 2],
    pub uv_tr: [f32; 2],
}

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

pub struct RenderingService<'a> {
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    image_receiver: tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>,
    redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,

    instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,

    internal_instance: Option<Instance>,
}

impl<'a> RenderingService<'a> {
    pub fn new<T>(
        window: T,
        config_receiver: tokio::sync::mpsc::Receiver<Config>,
        window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        redraw_requested_window_id_receiver: tokio::sync::mpsc::Receiver<WindowId>,
        image_receiver: tokio::sync::mpsc::Receiver<ImageLoadedEventArgs>,
    ) -> Self
    where
        T: Into<wgpu::SurfaceTarget<'a>>,
    {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).unwrap();

        Self {
            config_receiver,
            window_size_receiver,
            image_receiver,
            redraw_requested_window_id_receiver,
            instance,
            surface,
            internal_instance: None,
        }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(_config) = self.config_receiver.recv() => {},
            Some(args) = self.window_size_receiver.recv() => self.try_resize(args).await,
            Some(args) = self.image_receiver.recv() => self.apply_image(args).await,
            Some(window_id) = self.redraw_requested_window_id_receiver.recv() => self.redraw(window_id).await,
                                                        else => break,
                                                    );
        }
    }

    async fn try_resize(&mut self, args: WindowSizeChangedEventArgs) {
        if self.internal_instance.is_none() {
            self.internal_instance =
                Some(Self::create_instance(&self.instance, &self.surface).await);
        }

        let internal_instance = self.internal_instance.as_ref().unwrap();

        let adapter = &internal_instance.adapter;
        let surface = &self.surface;

        let swapchain_format = surface.get_capabilities(adapter).formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: args.width,
            height: args.height,
            present_mode: wgpu::PresentMode::Fifo,
            #[cfg(not(any(target_os = "macos", windows)))]
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            #[cfg(target_os = "macos")]
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            #[cfg(target_os = "windows")]
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let device = &internal_instance.device;
        surface.configure(device, &config);
    }

    async fn apply_image(&mut self, _args: ImageLoadedEventArgs) {}

    async fn apply_glyph_texture_patch(&mut self, patches: &GlyphTexturePatch) {
        if self.internal_instance.is_none() {
            self.internal_instance =
                Some(Self::create_instance(&self.instance, &self.surface).await);
        }

        let internal_instance = self.internal_instance.as_ref().unwrap();
        let queue = &internal_instance.queue;
        let glyph_texture = &internal_instance.glyph_texture;
    }

    async fn redraw(&mut self, _id: WindowId) {
        if self.internal_instance.is_none() {
            self.internal_instance =
                Some(Self::create_instance(&self.instance, &self.surface).await);
        }

        let instance = self.internal_instance.as_ref().unwrap();

        let device = &instance.device;
        let queue = &instance.queue;

        let Ok(frame) = self.surface.get_current_texture() else {
            return;
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 背景
        let is_background_enabled = if let Some(background_instance) = &instance.background_instance
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&background_instance.render_pipeline);
            render_pass.set_bind_group(0, &background_instance.bind_group, &[]);
            render_pass.set_vertex_buffer(0, instance.rect_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                instance.rect_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..6, 0, 0..1);

            true
        } else {
            false
        };

        // 文字描画
        if 0 < instance.character_count {
            let load_op = if is_background_enabled {
                // 背景があればロード処理
                wgpu::LoadOp::Load
            } else {
                // 背景がない場合はクリア処理
                wgpu::LoadOp::Clear(wgpu::Color::BLACK)
            };

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&instance.text_pipeline);
            render_pass.set_bind_group(0, &instance.text_bind_group, &[]);
            render_pass.set_vertex_buffer(0, instance.rect_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                instance.rect_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..6, 0, 0..instance.character_count);
        }

        queue.submit([command_encoder.finish()]);
    }

    async fn create_instance(instance: &wgpu::Instance, surface: &wgpu::Surface<'a>) -> Instance {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let format = surface.get_capabilities(&adapter).formats[0];

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

        let (text_pipeline, text_bind_group, glyph_texture) = {
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                    "../gfx/detail/char_rect.vs.wgsl"
                ))),
            });

            let pixel_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                    "../gfx/detail/char_rect.fs.wgsl"
                ))),
            });
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
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

            let glyph_texture = device.create_texture(&wgpu::TextureDescriptor {
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

            // 文字ごとの情報
            let character_storage_block = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: std::mem::size_of::<CharacterData>() as u64 * 32 * 1024,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
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
                        resource: wgpu::BindingResource::TextureView(&glyph_texture.create_view(
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

            (render_pipeline, bind_group, glyph_texture)
        };

        let rect_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[0.0f32, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let rect_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });

        Instance {
            device,
            queue,
            adapter,
            sampler,
            background_instance: None,
            text_pipeline,
            glyph_texture,
            text_bind_group,
            character_count: 0,
            rect_vertex_buffer,
            rect_index_buffer,
        }
    }

    fn create_background_instance(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sampler: &wgpu::Sampler,
        image: &image::DynamicImage,
        swapchain_format: wgpu::TextureFormat,
    ) -> BackgroundInstance {
        let vertex_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../gfx/detail/background.vs.wgsl"
            ))),
        });

        let pixel_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../gfx/detail/background.fs.wgsl"
            ))),
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
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &pixel_shader_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
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

        let view_constant_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });

        let material_constant_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });

        let (format, data): (wgpu::TextureFormat, &[u8]) = match image.color() {
            image::ColorType::Rgb8 => (wgpu::TextureFormat::Rgba8Unorm, &[]),
            image::ColorType::Rgba8 => {
                let image = image.as_rgba8().unwrap();
                (wgpu::TextureFormat::Rgba8Unorm, image.as_raw().as_slice())
            }
            // image::ColorType::L8 => todo!(),
            // image::ColorType::La8 => todo!(),
            // image::ColorType::L16 => todo!(),
            // image::ColorType::La16 => todo!(),
            // image::ColorType::Rgb16 => todo!(),
            // image::ColorType::Rgba16 => todo!(),
            // image::ColorType::Rgb32F => todo!(),
            // image::ColorType::Rgba32F => todo!(),
            _ => (wgpu::TextureFormat::R8Unorm, &[]),
        };

        let texture = device.create_texture_with_data(
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
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view_constant_buffer.as_entire_binding(),
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
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: material_constant_buffer.as_entire_binding(),
                },
            ],
        });

        BackgroundInstance {
            render_pipeline,
            bind_group_layout,
            bind_group,
            view_constant_buffer,
            material_constant_buffer,
            texture,
        }
    }
}

struct Instance {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::Adapter,

    #[allow(unused)]
    sampler: wgpu::Sampler,

    // 背景
    background_instance: Option<BackgroundInstance>,

    // テキスト描画
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group: wgpu::BindGroup,
    glyph_texture: wgpu::Texture,
    character_count: u32,

    rect_vertex_buffer: wgpu::Buffer,
    rect_index_buffer: wgpu::Buffer,
}

// TODO: 背景描画を実装する
#[allow(unused)]
struct BackgroundInstance {
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,

    bind_group_layout: wgpu::BindGroupLayout,
    view_constant_buffer: wgpu::Buffer,
    material_constant_buffer: wgpu::Buffer,
    texture: wgpu::Texture,
}
