use std::collections::HashMap;

use vw::{VirtualWindowId, VirtualWindowManager};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    window::{Window, WindowAttributes, WindowId},
};

fn main() {
    winit::event_loop::EventLoop::new()
        .unwrap()
        .run_app(&mut App::new())
        .unwrap();
}

struct App {
    windows: HashMap<WindowId, Window>,
    virtual_window_manager: VirtualWindowManager,
    window_tile_table: HashMap<VirtualWindowId, WindowId>,
    tile_window_table: HashMap<WindowId, VirtualWindowId>,
}

impl App {
    pub fn new() -> Self {
        let virtual_window_manager = vw::VirtualWindowManager::new();
        Self {
            windows: Default::default(),
            virtual_window_manager,
            window_tile_table: Default::default(),
            tile_window_table: Default::default(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_title("Hello World")
            .with_resizable(false)
            .with_position(PhysicalPosition::new(0.0, 0.0))
            .with_inner_size(PhysicalSize::new(640, 480));

        let window = event_loop.create_window(window_attributes.clone()).unwrap();
        let window_id = window.id();
        self.windows.insert(window_id, window);

        let virtual_window_id = self.virtual_window_manager.ids()[0];
        self.window_tile_table.insert(virtual_window_id, window_id);
        self.tile_window_table.insert(window_id, virtual_window_id);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::Moved(_) => {}
            WindowEvent::KeyboardInput {
                device_id, event, ..
            } => {
                let _ = device_id;
                if !event.state.is_pressed() {
                    return;
                }
                if event.repeat {
                    return;
                }
                let virtual_window_id = self.tile_window_table.get(&window_id).unwrap();
                let new_window_id = self
                    .virtual_window_manager
                    .split_horizontal(*virtual_window_id);

                let new_virtual_window = self
                    .virtual_window_manager
                    .try_get_virtual_window(new_window_id)
                    .unwrap();
                let window_attributes = WindowAttributes::default()
                    .with_title("Hello World")
                    .with_resizable(false)
                    .with_inner_size(PhysicalSize::new(
                        new_virtual_window.width(),
                        new_virtual_window.height(),
                    ));

                let window = event_loop.create_window(window_attributes.clone()).unwrap();
                let window_id = window.id();
                self.windows.insert(window_id, window);

                self.window_tile_table.insert(new_window_id, window_id);
                self.tile_window_table.insert(window_id, new_window_id);

                for virtual_window_id in self.virtual_window_manager.ids() {
                    let Some(window_id) = self.window_tile_table.get(&virtual_window_id) else {
                        continue;
                    };

                    let Some(window) = self.windows.get(window_id) else {
                        continue;
                    };

                    let Some(virtual_window) = self
                        .virtual_window_manager
                        .try_get_virtual_window(*virtual_window_id)
                    else {
                        continue;
                    };

                    println!(
                        "(w, h, x, y) = ({}, {}, {}, {})",
                        virtual_window.width(),
                        virtual_window.height(),
                        virtual_window.x(),
                        virtual_window.y()
                    );
                    window.set_outer_position(PhysicalPosition::new(
                        virtual_window.x(),
                        virtual_window.y(),
                    ));
                    if let Some(_) = window.request_inner_size(PhysicalSize::new(
                        virtual_window.width(),
                        virtual_window.height(),
                    )) {}
                }
            }
            WindowEvent::CloseRequested => {
                if self.virtual_window_manager.ids().len() > 1 {
                    return;
                }

                event_loop.exit();
            }
            _ => {}
        }
    }
}
