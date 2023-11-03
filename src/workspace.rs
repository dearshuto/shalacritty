use std::{collections::HashMap, sync::Arc};

use alacritty_terminal::event_loop::{EventLoopSender, Msg};
use winit::{event_loop::EventLoopWindowTarget, window::WindowId};

use crate::{
    gfx::{ContentPlotter, GlyphManager, Renderer},
    tty::{TeletypeId, TeletypeManager},
    window::WindowManager,
    ConfigService,
};

pub struct Workspace<'a> {
    instance: wgpu::Instance,
    #[allow(dead_code)]
    config_service: Arc<ConfigService>,
    glyph_manager: GlyphManager,
    teletype_manager: TeletypeManager,
    window_manager: WindowManager,
    content_plotter: ContentPlotter,
    renderer: Renderer<'a>,
    window_tty_table: HashMap<WindowId, Vec<TeletypeId>>,
    sender: Option<EventLoopSender>,
}

impl<'a> Workspace<'a> {
    pub fn new() -> Self {
        let instance = wgpu::Instance::default();
        let config_service = Arc::new(ConfigService::new());
        let glyph_manager = GlyphManager::new();
        let teletype_manager = TeletypeManager::new();
        let window_manager = WindowManager::new();
        let content_plotter = ContentPlotter::new();
        let renderer = Renderer::new(Arc::clone(&config_service));

        Self {
            instance,
            config_service,
            glyph_manager,
            teletype_manager,
            window_manager,
            content_plotter,
            renderer,
            window_tty_table: HashMap::default(),
            sender: None,
        }
    }

    pub async fn spawn_window<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) {
        let id = self.window_manager.create_window(event_loop).await;
        let window = self.window_manager.try_get_window(id).unwrap();
        self.renderer.register(id, &self.instance, window).await;

        let (tty_id, sender) = self.teletype_manager.create_teletype();
        self.window_tty_table.insert(id, vec![tty_id]);
        self.sender = Some(sender);
    }

    pub fn update(&mut self) {
        // 表示する要素が更新されていたら描画する要素に反映する
        for (window_id, value) in &self.window_tty_table {
            for id in value {
                // 変化がなければなにもしない
                if !self.teletype_manager.is_dirty(*id) {
                    continue;
                }

                // レンダラーに反映
                self.teletype_manager.get_content(*id, |c| {
                    let diff = self
                        .content_plotter
                        .calculate_diff(c, &mut self.glyph_manager);
                    self.renderer.update(*window_id, diff);
                });

                // ダーティフラグを解除
                self.teletype_manager.clear_dirty(*id);

                // 最描画要求
                let Some(window) = self.window_manager.try_get_window(*window_id) else {
                    return;
                };
                window.request_redraw();
            }
        }
    }

    pub fn render(&mut self, id: WindowId) {
        self.renderer.render(id);
    }

    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        self.renderer.resize(id, width, height);

        let Some(tty_ids) = self.window_tty_table.get(&id) else {
            return;
        };

        for tty_id in tty_ids {
            self.teletype_manager.is_dirty(*tty_id);
            self.teletype_manager.get_content(*tty_id, |c| {
                let diff = self
                    .content_plotter
                    .calculate_diff(c, &mut self.glyph_manager);
                self.renderer.update(id, diff);
            });
        }
    }

    pub fn send(&mut self, _id: WindowId, text: &str) {
        let mut bytes = Vec::with_capacity(text.len() + 1);
        bytes.extend_from_slice(text.as_bytes());
        if text.is_empty() {
            bytes.push(b'\x1b');
        }
        self.sender.as_mut().unwrap().send(Msg::Input(bytes.into()));
    }
}
