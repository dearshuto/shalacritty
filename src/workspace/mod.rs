mod virtual_window_manager;

pub use virtual_window_manager::{VirtualWindowId, VirtualWindowManager};

use std::{collections::HashMap, sync::Arc};

use alacritty_terminal::{
    event::WindowSize,
    event_loop::{EventLoopSender, Msg},
};
use winit::{event_loop::EventLoopWindowTarget, window::WindowId};

use crate::{
    gfx::{ContentPlotter, GlyphManager, Renderer, RendererUpdateParams},
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

    #[allow(dead_code)]
    virtual_window_manager: VirtualWindowManager,

    // VirtualWindow -> Tty
    virtual_window_tty_table: HashMap<VirtualWindowId, TeletypeId>,

    // 操作対象となっているウィンドウ
    active_window_id: Option<VirtualWindowId>,
}

impl<'a> Workspace<'a> {
    pub fn new() -> Self {
        let instance = wgpu::Instance::default();
        let config_service = Arc::new(ConfigService::new());
        let glyph_manager = GlyphManager::new();
        let teletype_manager = TeletypeManager::new();
        let window_manager = WindowManager::new();
        let content_plotter = ContentPlotter::new();
        let renderer = Renderer::new();

        // ウィンドウを分割した仮想的な領域
        let mut virtual_window_manager = VirtualWindowManager::new();
        let id = virtual_window_manager.spawn_virtual_window(64, 64);
        let _virtual_window = virtual_window_manager.try_get_window(id);

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
            virtual_window_manager,
            virtual_window_tty_table: HashMap::default(),
            active_window_id: None,
        }
    }

    pub async fn spawn_window<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) {
        let id = self.window_manager.create_window(event_loop).await;
        let window = self.window_manager.try_get_window(id).unwrap();
        let window_size = window.inner_size();
        self.renderer.register(id, &self.instance, window).await;

        let (tty_id, sender) = self.teletype_manager.create_teletype();
        self.window_tty_table.insert(id, vec![tty_id]);
        self.sender = Some(sender);

        // シェルを表示する領域
        let virtual_window_id = self.virtual_window_manager.spawn_virtual_window(64, 64);
        self.virtual_window_tty_table
            .insert(virtual_window_id, tty_id);

        self.active_window_id = Some(virtual_window_id);

        // 初期サイズ反映
        self.resize(id, window_size.width, window_size.height);
    }

    pub fn update(&mut self) {
        self.teletype_manager.update();

        self.virtual_window_manager.uodate();

        // 表示する要素が更新されていたら描画する要素に反映する
        for (window_id, value) in &self.window_tty_table {
            // 最描画要求
            let Some(window) = self.window_manager.try_get_window(*window_id) else {
                return;
            };
            window.request_redraw();

            for id in value {
                // 変化がなければなにもしない
                if !self.teletype_manager.is_dirty(*id) {
                    continue;
                }

                // レンダラーに反映
                self.teletype_manager.get_content(*id, |c| {
                    let config = self.config_service.read().unwrap();
                    let diff = self
                        .content_plotter
                        .calculate_diff(c, &mut self.glyph_manager);
                    let update_params = RendererUpdateParams::new()
                        .with_diff(diff)
                        .with_background_color(config.background)
                        .with_image_alpha(config.image_alpha)
                        .with_image_path(config.image.clone());
                    self.renderer.update(*window_id, update_params);
                });

                // ダーティフラグを解除
                self.teletype_manager.clear_dirty(*id);
            }
        }
    }

    pub fn render(&mut self, id: WindowId) {
        self.renderer.render(id);
    }

    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        // 仮想ウインドウにリサイズを反映
        self.virtual_window_manager.resize(width, height);

        self.renderer.resize(id, width, height);

        let Some(tty_ids) = self.window_tty_table.get(&id) else {
            return;
        };

        for tty_id in tty_ids {
            self.teletype_manager.is_dirty(*tty_id);

            self.teletype_manager.get_content(*tty_id, |c| {
                let config = self.config_service.read().unwrap();
                let diff = self
                    .content_plotter
                    .calculate_diff(c, &mut self.glyph_manager);
                let update_params = RendererUpdateParams::new()
                    .with_diff(diff)
                    .with_image_alpha(config.image_alpha)
                    .with_image_path(config.image.clone())
                    .with_background_color(config.background);
                self.renderer.update(id, update_params);
            });
        }

        // TODO: tty のリサイズ
        self.sender
            .as_mut()
            .unwrap()
            .send(Msg::Resize(WindowSize {
                num_lines: 64,
                num_cols: 64,
                cell_width: 8,
                cell_height: 8,
            }))
            .unwrap();

        // 最描画要求
        let Some(window) = self.window_manager.try_get_window(id) else {
            return;
        };
        window.request_redraw();
    }

    pub fn send(&mut self, _id: WindowId, text: &str) {
        let mut bytes = Vec::with_capacity(text.len() + 1);
        bytes.extend_from_slice(text.as_bytes());
        if text.is_empty() {
            bytes.push(b'\x1b');
        }

        let send_data: std::borrow::Cow<[u8]> = match text {
            // 上
            "\u{f700}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x41]),
            // 下
            "\u{f701}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x42]),
            // 左
            "\u{f702}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x44]),
            // 右
            "\u{f703}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x43]),
            _ => std::borrow::Cow::Owned(bytes),
        };

        self.sender
            .as_mut()
            .unwrap()
            .send(Msg::Input(send_data))
            .unwrap();
    }

    pub fn is_empty(&self) -> bool {
        self.teletype_manager.is_empty()
    }
}
