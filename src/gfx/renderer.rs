use std::{collections::HashMap, num::NonZeroU64};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use wgpu::{include_spirv_raw, util::DeviceExt};

use crate::window::WindowId;

#[allow(dead_code)]
pub struct Renderer {
    device_table: HashMap<WindowId, wgpu::Device>,
    queue_table: HashMap<WindowId, wgpu::Queue>,
    adapter_table: HashMap<WindowId, wgpu::Adapter>,
    surface_table: HashMap<WindowId, wgpu::Surface>,
    pipelie_table: HashMap<WindowId, wgpu::RenderPipeline>,
    vertex_buffer_table: HashMap<WindowId, wgpu::Buffer>,
    index_buffer_table: HashMap<WindowId, wgpu::Buffer>,
    bind_group_table: HashMap<WindowId, wgpu::BindGroup>,
    character_storage_block_table: HashMap<WindowId, wgpu::Buffer>,
}

impl Renderer {
    #[allow(dead_code)]
    pub fn new() -> Self {
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
        }
    }

    pub async fn register<TWindow>(
        &mut self,
        id: WindowId,
        instance: &wgpu::Instance,
        window: &TWindow,
    ) where
        TWindow: HasRawWindowHandle + HasRawDisplayHandle,
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
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<f32>() as u64 * 6),
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
                targets: &[Some(config.view_formats[0].into())],
            }),
            multiview: None,
        });

        // 頂点バッファー
        let vertrex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[-0.5f32, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // インデックスバッファー
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });

        // 文字ごとの情報
        let character_storage_block =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[
                    0.5f32, 0.0, -0.3, 0.0, 0.0, 0.5, 0.0, 0.0, //
                    0.5f32, 0.0, 0.3, 0.0, 0.0, 0.5, 0.0, 0.0, //
                ]),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: character_storage_block.as_entire_binding(),
            }],
        });

        self.device_table.insert(id.clone(), device);
        self.queue_table.insert(id.clone(), queue);
        self.adapter_table.insert(id, adapter);
        self.surface_table.insert(id.clone(), surface);
        self.pipelie_table.insert(id.clone(), render_pipeline);
        self.vertex_buffer_table.insert(id.clone(), vertrex_buffer);
        self.index_buffer_table.insert(id.clone(), index_buffer);
        self.character_storage_block_table
            .insert(id.clone(), character_storage_block);
        self.bind_group_table.insert(id.clone(), bind_group);
    }

    pub fn resize(&self, id: WindowId, width: u32, height: u32) {
        let device = self.device_table.get(&id).unwrap();
        let surface = self.surface_table.get(&id).unwrap();
        let adapter = self.adapter_table.get(&id).unwrap();
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(device, &config);
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
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.draw_indexed(0..6, 0, 0..2);
        }

        queue.submit(Some(command_encoder.finish()));
        frame.present();
    }
}
