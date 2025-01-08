use std::{collections::HashMap, time::Duration};

use raw_window_handle::HasDisplayHandle;
use wgpu::rwh::HasWindowHandle;
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopBuilder},
    window::{Window, WindowAttributes, WindowId},
};

fn main() {
    let event_loop = EventLoopBuilder::default().build().unwrap();

    event_loop
        .run_app(&mut App::new(term_gfx::Renderer::new()))
        .unwrap();
}

struct App<TBackend: term_gfx::IBackend> {
    window: Option<Window>,
    window_render_target_table: HashMap<WindowId, TBackend::RenderTargetId>,
    renderer: term_gfx::Renderer<TBackend>,
}

impl<TBackend: term_gfx::IBackend> App<TBackend> {
    pub fn new(renderer: term_gfx::Renderer<TBackend>) -> Self {
        Self {
            window: None,
            window_render_target_table: HashMap::default(),
            renderer,
        }
    }
}

impl<TBackend: term_gfx::IBackend> ApplicationHandler for App<TBackend> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // レンダラーのバックエンドで実装簡略化のために 640x480 を決め打ちしているのでここでも制限をかける
        let window_attribute = WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(640, 480))
            .with_resizable(false);
        let window = event_loop.create_window(window_attribute).unwrap();
        let window_handle = window.window_handle().unwrap();
        let display_handle = window.display_handle().unwrap();
        let id = self
            .renderer
            .register_surface(window_handle, display_handle)
            .unwrap();

        self.window_render_target_table.insert(window.id(), id);
        self.window = Some(window);

        // Renderer のバックエンドの VSync に期待するので Poll でよい
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        let _ = (event_loop, cause);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // 適当な FPS で再描画要求を出しておく
        // TODO: 更新タイミングの厳密な管理
        if let Some(window) = &self.window {
            window.request_redraw();
            std::thread::sleep(Duration::from_millis(20));
        }

        match event {
            WindowEvent::RedrawRequested => {
                let Some(_target_id) = self.window_render_target_table.get(&window_id) else {
                    return;
                };

                self.renderer.render();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        };
    }
}
