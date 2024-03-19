use std::{collections::HashMap, path::Path, sync::Arc};

use wgpu::WasmNotSendSync;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::WindowId,
};

use super::{
    content_plotter::Diff,
    detail::{BackgroundRenderer, CursorRenderer, ScanBufferRenderer, TextRenderer},
};

pub struct RendererUpdateParams<TPath: AsRef<Path>> {
    width: u32,
    height: u32,
    background_color: Option<[f32; 4]>,
    diff: Diff,
    image_path: Option<TPath>,
    image_alpha: Option<f32>,
}

impl<TPath: AsRef<Path>> RendererUpdateParams<TPath> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            background_color: None,
            diff: Diff::default(),
            image_path: None,
            image_alpha: None,
        }
    }

    pub fn with_diff(mut self, diff: Diff) -> Self {
        self.diff = diff;
        self
    }

    pub fn with_image_alpha(mut self, alpha: Option<f32>) -> Self {
        self.image_alpha = alpha;
        self
    }

    pub fn with_background_color(mut self, color: Option<[f32; 4]>) -> Self {
        self.background_color = color;
        self
    }

    pub fn with_image_path(mut self, path: Option<TPath>) -> Self {
        self.image_path = path;
        self
    }
}

pub struct Renderer<'a> {
    device_table: HashMap<WindowId, wgpu::Device>,
    queue_table: HashMap<WindowId, wgpu::Queue>,
    adapter_table: HashMap<WindowId, wgpu::Adapter>,
    surface_table: HashMap<WindowId, wgpu::Surface<'a>>,

    // テキスト描画
    text_renderer: TextRenderer<'a>,

    // カーソル
    cursor_renderer: CursorRenderer<'a>,

    // 背景
    background_renderer: BackgroundRenderer<'a>,

    // スキャンバッファーに表示
    scan_buffer_renderer: ScanBufferRenderer<'a>,

    // 背景色
    background_color: [f32; 4],
}

impl<'a> Renderer<'a> {
    pub fn new() -> Self {
        Self {
            device_table: Default::default(),
            queue_table: Default::default(),
            adapter_table: Default::default(),
            surface_table: Default::default(),

            // テキスト描画
            text_renderer: TextRenderer::new(),

            // カーソル
            cursor_renderer: CursorRenderer::new(),

            // 背景
            background_renderer: BackgroundRenderer::new(),

            // スキャンバッファー描画
            scan_buffer_renderer: ScanBufferRenderer::new(),

            // 背景色
            background_color: [0.3, 0.4, 0.5, 0.5],
        }
    }

    pub async fn register<TWindow>(
        &mut self,
        id: WindowId,
        instance: &wgpu::Instance,
        window: Arc<TWindow>,
    ) where
        TWindow: HasWindowHandle + HasDisplayHandle + WasmNotSendSync + 'a,
    {
        let surface = instance.create_surface(window).unwrap();
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
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .unwrap();

        let swapchain_capabilities = surface.get_capabilities(&adapter);

        let swapchain_format = swapchain_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
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
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // テキスト描画
        self.text_renderer
            .register(id, &device, config.format)
            .await;

        // カーソル描画
        self.cursor_renderer
            .register(id, &device, &queue, config.format);

        // スキャンバッファー描画
        self.scan_buffer_renderer
            .register(id, &device, swapchain_format);

        self.device_table.insert(id, device);
        self.queue_table.insert(id, queue);
        self.adapter_table.insert(id, adapter);
        self.surface_table.insert(id, surface);
    }

    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        let device = self.device_table.get(&id).unwrap();
        let queue = self.queue_table.get(&id).unwrap();
        let surface = self.surface_table.get(&id).unwrap();
        let adapter = self.adapter_table.get(&id).unwrap();
        let swapchain_capabilities = surface.get_capabilities(adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
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
            desired_maximum_frame_latency: 2,
        };
        surface.configure(device, &config);

        // カーソル
        self.cursor_renderer.resize(width, height);

        // 背景描画
        // TODO: プラグイン化
        self.background_renderer.resize(id, queue, width, height);

        // スキャンバッファー描画
        self.scan_buffer_renderer.resize(id, queue, width, height);
    }

    pub fn update<TPath>(&mut self, id: WindowId, render_update_params: RendererUpdateParams<TPath>)
    where
        TPath: AsRef<Path>,
    {
        if let Some(background_color) = render_update_params.background_color {
            self.background_color = background_color;
        }

        if let (
            Some(device),
            Some(queue),
            Some(surface),
            Some(adapter),
            Some(image_path),
            Some(alpha),
        ) = (
            self.device_table.get(&id),
            self.queue_table.get(&id),
            self.surface_table.get(&id),
            self.adapter_table.get(&id),
            render_update_params.image_path,
            render_update_params.image_alpha,
        ) {
            let texture_format = surface.get_capabilities(adapter).formats[0];
            self.background_renderer
                .register(id, device, queue, texture_format, image_path, alpha);
            self.background_renderer.resize(
                id,
                queue,
                render_update_params.width,
                render_update_params.height,
            );
        }

        let queue = self.queue_table.get(&id).unwrap();
        self.text_renderer
            .update(queue, id, &render_update_params.diff);

        // カーソルレンダラーの更新
        self.cursor_renderer
            .update(id, &render_update_params.diff, queue);
    }

    pub fn render(&self, id: WindowId) {
        let device = self.device_table.get(&id).unwrap();
        let queue = self.queue_table.get(&id).unwrap();
        let surface = self.surface_table.get(&id).unwrap();

        let frame = surface.get_current_texture().unwrap();

        let view = self.scan_buffer_renderer.create_view(id);

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 背景描画
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.background_color[0] as f64,
                            g: self.background_color[1] as f64,
                            b: self.background_color[2] as f64,
                            a: self.background_color[3] as f64,
                        }),
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
                frame.texture.size().width as f32,
                frame.texture.size().height as f32,
                0.0,
                1.0,
            );
            self.background_renderer.render(id, render_pass);
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
            render_pass.set_viewport(
                0.0,
                0.0,
                frame.texture.size().width as f32,
                frame.texture.size().height as f32,
                0.0,
                1.0,
            );
            self.text_renderer.render(id, render_pass);
        }

        // カーソル描画
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
            render_pass.set_viewport(
                0.0,
                0.0,
                frame.texture.size().width as f32,
                frame.texture.size().height as f32,
                0.0,
                1.0,
            );
            self.cursor_renderer.render(id, render_pass);
        }

        // スキャンバッファーにコピー
        // 1. ドットバイドット対応
        // 2. 座標系調整
        {
            let scan_buffer_view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &scan_buffer_view,
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
                frame.texture.size().width as f32,
                frame.texture.size().height as f32,
                0.0,
                1.0,
            );

            self.scan_buffer_renderer.render(id, render_pass);
        }

        queue.submit(Some(command_encoder.finish()));
        frame.present();
    }
}
