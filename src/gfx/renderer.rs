use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use uuid::Uuid;
use wgpu::WasmNotSendSync;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::WindowId,
};

use super::{
    content_plotter::Diff,
    detail::{BackgroundRenderer, CursorRenderer, TextRenderer},
};

#[derive(Debug, Hash, Clone, Copy)]
pub struct BackgroundId {
    id: Uuid,
}

pub struct RendererUpdateParams<TPath: AsRef<Path>> {
    background_color: Option<[f32; 4]>,
    diff: Diff,
    image_path: Option<TPath>,
    image_alpha: Option<f32>,
}

impl Default for RendererUpdateParams<PathBuf> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TPath: AsRef<Path>> RendererUpdateParams<TPath> {
    pub fn new() -> Self {
        Self {
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

    pub fn with_image_alpha(mut self, alpha: f32) -> Self {
        self.image_alpha = Some(alpha);
        self
    }

    pub fn with_background_color(mut self, color: [f32; 4]) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn with_image_path(mut self, path: TPath) -> Self {
        self.image_path = Some(path);
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
                    required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .unwrap();

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
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // テキスト描画
        self.text_renderer
            .register(id, &device, config.format)
            .await;

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
            desired_maximum_frame_latency: 2,
        };
        surface.configure(device, &config);

        // 背景描画
        // TODO: プラグイン化
        self.background_renderer.resize(id, queue, width, height);
    }

    pub fn update<TPath>(&mut self, id: WindowId, render_update_params: RendererUpdateParams<TPath>)
    where
        TPath: AsRef<Path>,
    {
        if let Some(background_color) = render_update_params.background_color {
            self.background_color = background_color;
        }

        let device = self.device_table.get(&id).unwrap();
        let queue = self.queue_table.get(&id).unwrap();
        self.text_renderer
            .update(queue, id, &render_update_params.diff);

        // 背景レンダラーの更新
        let surface = self.surface_table.get(&id).unwrap();
        let adapter = self.adapter_table.get(&id).unwrap();
        let swapchain_capabilities = surface.get_capabilities(adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        if let (Some(image_path), Some(image_alpha)) = (
            render_update_params.image_path,
            render_update_params.image_alpha,
        ) {
            if image_path.as_ref().exists() {
                self.background_renderer.update(
                    id,
                    device,
                    queue,
                    swapchain_format,
                    image_path,
                    image_alpha,
                );
            }
        }

        // カーソルレンダラーの更新
        self.cursor_renderer
            .update(id, &render_update_params.diff, queue);
    }

    pub fn render(&self, id: WindowId) {
        let device = self.device_table.get(&id).unwrap();
        let queue = self.queue_table.get(&id).unwrap();
        let surface = self.surface_table.get(&id).unwrap();

        let frame = surface.get_current_texture().unwrap();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // 背景描画
        {
            let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            self.text_renderer.render(id, render_pass);
        }

        queue.submit(Some(command_encoder.finish()));
        frame.present();
    }

    #[allow(dead_code)]
    pub fn register_background<TPath>(&mut self, _path: TPath) -> BackgroundId
    where
        TPath: AsRef<Path>,
    {
        BackgroundId { id: Uuid::new_v4() }
    }
}
