use std::{collections::HashMap, num::NonZeroU64, path::Path, sync::Arc};

use wgpu::{include_spirv_raw, util::DeviceExt};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::WindowId,
};

use crate::ConfigService;

use super::{
    content_plotter::Diff,
    detail::{BackgroundRenderer, CursorRenderer},
};

#[repr(C)]
#[derive(Debug)]
struct CharacterData {
    transform0: [f32; 4],
    transform1: [f32; 4],
    fore_ground_color: [f32; 4],
    uv_bl: [f32; 2],
    uv_tr: [f32; 2],
}

#[allow(dead_code)]
pub struct Renderer<'a> {
    device_table: HashMap<WindowId, wgpu::Device>,
    queue_table: HashMap<WindowId, wgpu::Queue>,
    adapter_table: HashMap<WindowId, wgpu::Adapter>,
    surface_table: HashMap<WindowId, wgpu::Surface>,
    pipelie_table: HashMap<WindowId, wgpu::RenderPipeline>,
    vertex_buffer_table: HashMap<WindowId, wgpu::Buffer>,
    index_buffer_table: HashMap<WindowId, wgpu::Buffer>,
    bind_group_table: HashMap<WindowId, wgpu::BindGroup>,
    character_storage_block_table: HashMap<WindowId, wgpu::Buffer>,
    sampler_table: HashMap<WindowId, wgpu::Sampler>,
    glyph_texture: Option<wgpu::Texture>,

    // カーソル
    cursor_renderer: CursorRenderer<'a>,

    // 背景
    background_renderer: BackgroundRenderer<'a>,

    // 設定
    config: Arc<ConfigService>,
}

impl<'a> Renderer<'a> {
    #[allow(dead_code)]
    pub fn new(config: Arc<ConfigService>) -> Self {
        Self {
            device_table: Default::default(),
            queue_table: Default::default(),
            adapter_table: Default::default(),
            surface_table: Default::default(),
            pipelie_table: Default::default(),
            vertex_buffer_table: Default::default(),
            index_buffer_table: HashMap::default(),
            bind_group_table: Default::default(),
            character_storage_block_table: Default::default(),
            sampler_table: HashMap::default(),
            glyph_texture: None,

            // カーソル
            cursor_renderer: CursorRenderer::new(),

            // 背景
            background_renderer: BackgroundRenderer::new(),

            // 設定
            config,
        }
    }

    pub async fn register<TWindow>(
        &mut self,
        id: WindowId,
        instance: &wgpu::Instance,
        window: &TWindow,
    ) where
        TWindow: HasWindowHandle + HasDisplayHandle,
    {
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                    limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .unwrap();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            std::mem::size_of::<CharacterData>() as u64 * 1024,
                        ),
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

        let vertex_shader_module_spirv = include_spirv_raw!("char_rect.vs.spv");
        let vertex_shader_module =
            unsafe { device.create_shader_module_spirv(&vertex_shader_module_spirv) };

        let pixel_shader_module_spirv = include_spirv_raw!("char_rect.fs.spv");
        let _pixel_shader_module =
            unsafe { device.create_shader_module_spirv(&pixel_shader_module_spirv) };

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: 640,
            height: 480,
            present_mode: wgpu::PresentMode::Fifo,
            #[cfg(not(any(target_os = "macos", windows)))]
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            #[cfg(target_os = "macos")]
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            #[cfg(target_os = "windows")]
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![swapchain_format],
        };
        surface.configure(&device, &config);

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
                module: &_pixel_shader_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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
            size: std::mem::size_of::<CharacterData>() as u64 * 4 * 1024,
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

        // 背景描画
        // TODO: プラグイン化
        self.background_renderer
            .register(id, &device, &queue, config.format);

        // カーソル描画
        self.cursor_renderer
            .register(id, &device, &queue, config.format);

        self.device_table.insert(id, device);
        self.queue_table.insert(id, queue);
        self.adapter_table.insert(id, adapter);
        self.surface_table.insert(id, surface);
        self.pipelie_table.insert(id, render_pipeline);
        self.vertex_buffer_table.insert(id, vertrex_buffer);
        self.index_buffer_table.insert(id, index_buffer);
        self.character_storage_block_table
            .insert(id, character_storage_block);
        self.bind_group_table.insert(id, bind_group);
        self.sampler_table.insert(id, sampler);
        self.glyph_texture = Some(texture);
    }

    pub fn resize(&self, id: WindowId, width: u32, height: u32) {
        let device = self.device_table.get(&id).unwrap();
        let queue = self.queue_table.get(&id).unwrap();
        let surface = self.surface_table.get(&id).unwrap();
        let adapter = self.adapter_table.get(&id).unwrap();
        let swapchain_capabilities = surface.get_capabilities(adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: width as u32,
            height: height as u32,
            present_mode: wgpu::PresentMode::Fifo,
            #[cfg(not(any(target_os = "macos", windows)))]
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            #[cfg(target_os = "macos")]
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            #[cfg(target_os = "windows")]
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(device, &config);

        // 背景描画
        // TODO: プラグイン化
        self.background_renderer.resize(id, queue, width, height);
    }

    pub fn update(&mut self, id: WindowId, diff: Diff) {
        let device = self.device_table.get(&id).unwrap();
        let queue = self.queue_table.get(&id).unwrap();
        let buffer = self.character_storage_block_table.get(&id).unwrap();
        let data = diff
            .character_info_array()
            .iter()
            .map(|info| {
                let t = info.transform;
                CharacterData {
                    transform0: [t[0], t[1], t[2], 0.0],
                    transform1: [t[3], t[4], t[5], 0.0],
                    fore_ground_color: info.fore_ground_color,
                    uv_bl: [info.uv0[0], info.uv0[1]],
                    uv_tr: [info.uv1[0], info.uv1[1]],
                }
            })
            .collect::<Vec<CharacterData>>();

        if !data.is_empty() {
            let binary = unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const _ as *const u8,
                    std::mem::size_of::<CharacterData>() * data.len(),
                )
            };

            queue.write_buffer(buffer, 0, binary);
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

        // 背景レンダラーの更新
        let surface = self.surface_table.get(&id).unwrap();
        let adapter = self.adapter_table.get(&id).unwrap();
        let swapchain_capabilities = surface.get_capabilities(adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let config = self.config.read().unwrap();
        let image_path = &config.image;
        let image_alpha = config.image_alpha;
        if Path::new(image_path).exists() {
            self.background_renderer.update(
                id,
                device,
                queue,
                swapchain_format,
                image_path,
                image_alpha,
            );
        }

        // カーソルレンダラーの更新
        self.cursor_renderer.update(id, &diff, queue);
    }

    pub fn render(&self, id: WindowId) {
        let device = self.device_table.get(&id).unwrap();
        let queue = self.queue_table.get(&id).unwrap();
        let surface = self.surface_table.get(&id).unwrap();
        let render_pipeline = self.pipelie_table.get(&id).unwrap();
        let vertex_buffer = self.vertex_buffer_table.get(&id).unwrap();
        let index_buffer = self.index_buffer_table.get(&id).unwrap();
        let bind_group = self.bind_group_table.get(&id).unwrap();

        let frame = surface.get_current_texture().unwrap();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 背景描画
        {
            let config = self.config.read().unwrap();
            let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: config.background[0] as f64,
                            g: config.background[1] as f64,
                            b: config.background[2] as f64,
                            a: config.background[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.background_renderer.render(id, render_pass);
        }

        // カーソル描画
        {
            let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            self.cursor_renderer.render(id, render_pass);
        }

        // 文字描画
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, bind_group, &[]);

            // TODO: 文字数は外部から受け取るようにする
            // 4096 は 64x64 の領域に文字が入るようにしている
            render_pass.draw_indexed(0..6, 0, 0..4096);
        }

        queue.submit(Some(command_encoder.finish()));
        frame.present();
    }
}
