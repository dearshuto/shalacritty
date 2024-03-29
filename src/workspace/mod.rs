mod detail;
mod diff_calculator;

use std::{borrow::Cow, collections::HashMap, sync::Arc};

use alacritty_terminal::{
    event::WindowSize,
    event_loop::{EventLoopSender, Msg},
};
use winit::{event_loop::EventLoopWindowTarget, window::WindowId};

use crate::{
    gfx::{ContentPlotter, GlyphManager, Renderer, RendererUpdateParams},

    // 本体は detail 以下にはアクセスさせたくない
    // multiplexers モジュールへの移植途中の互換性保持として直接参照している
    multiplexers::{
        detail::{VirtualWindowId, VirtualWindowManager},
        TileManager,
    },

    tty::{TeletypeId, TeletypeManager},
    window::WindowManager,
    Config,
    ConfigService,
};

use self::detail::MultiplexersAdapter;

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

    tile_manager: TileManager<MultiplexersAdapter>,

    old_config: Option<Config>,
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

        let (tile_manager, _id) = TileManager::new(MultiplexersAdapter::new());

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
            tile_manager,
            old_config: None,
        }
    }

    pub async fn spawn_window<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) {
        let id = self.window_manager.create_window(event_loop).await;
        let window = self.window_manager.try_get_window(id).unwrap();
        let window_size = window.inner_size();
        self.renderer.register(id, &self.instance, window).await;
        self.renderer
            .resize(id, window_size.width, window_size.height);

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
        self.tile_manager.update();

        for ptr_write in self.teletype_manager.consume_ptr_write() {
            self.sender
                .as_mut()
                .unwrap()
                .send(Msg::Input(Cow::Owned(ptr_write)))
                .unwrap();
        }

        self.virtual_window_manager.uodate();

        // 表示する要素が更新されていたら描画する要素に反映する
        for (window_id, value) in &self.window_tty_table {
            // 最描画要求
            let Some(window) = self.window_manager.try_get_window(*window_id) else {
                return;
            };
            window.request_redraw();

            let (background, image_alpha, image_path) = if self.old_config.is_none() {
                let config = self.config_service.read().unwrap();
                self.old_config = Some(config.clone());

                (
                    Some(config.background.clear_color),
                    Some(config.image_alpha),
                    Some(config.image.clone()),
                )
            } else {
                let old_config = self.old_config.as_ref().unwrap();
                let config = self.config_service.read().unwrap();
                let background =
                    if old_config.background.clear_color == config.background.clear_color {
                        None
                    } else {
                        Some(config.background.clear_color)
                    };
                let alpha = if old_config.image_alpha == config.image_alpha {
                    None
                } else {
                    Some(config.image_alpha)
                };
                let image = if old_config.image == config.image {
                    None
                } else {
                    Some(config.image.clone())
                };
                self.old_config = Some(config.clone());

                (background, alpha, image)
            };

            for id in value {
                // 変化がなければなにもしない
                if !self.teletype_manager.is_dirty(*id) {
                    continue;
                }

                // レンダラーに反映
                self.teletype_manager.get_content(*id, |c| {
                    let diff = self.content_plotter.calculate_diff(
                        c,
                        &mut self.glyph_manager,
                        (window.inner_size().width, window.inner_size().height),
                    );
                    let update_params = RendererUpdateParams::new(
                        window.inner_size().width,
                        window.inner_size().height,
                    )
                    .with_diff(diff)
                    .with_background_color(background)
                    .with_image_alpha(image_alpha)
                    .with_image_path(image_path.clone());
                    self.renderer.update(*window_id, update_params);
                });

                // ダーティフラグを解除
                self.teletype_manager.clear_dirty(*id);
            }
        }

        // TODO: ↑ は背景の変更だけにして処理を切り離したい
        // tty の差分をレンダラーに反映
        for window_id in self.window_manager.ids() {
            let Some(window) = self.window_manager.try_get_window(*window_id) else {
                continue;
            };

            let Some(teletype_ids) = self.window_tty_table.get(window_id) else {
                continue;
            };

            for teletype_id in teletype_ids {
                if !self.teletype_manager.is_dirty(*teletype_id) {
                    continue;
                }

                // レンダラーに反映
                self.teletype_manager.get_content(*teletype_id, |c| {
                    let diff = self.content_plotter.calculate_diff(
                        c,
                        &mut self.glyph_manager,
                        (window.inner_size().width, window.inner_size().height),
                    );
                    let update_params = RendererUpdateParams::<String>::new(
                        window.inner_size().width,
                        window.inner_size().height,
                    )
                    .with_diff(diff);
                    self.renderer.update(*window_id, update_params);
                });

                // ダーティフラグを解除
                self.teletype_manager.clear_dirty(*teletype_id);
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
                let diff = self.content_plotter.calculate_diff(
                    c,
                    &mut self.glyph_manager,
                    (width, height),
                );
                let update_params =
                    RendererUpdateParams::<String>::new(width, height).with_diff(diff);
                self.renderer.update(id, update_params);
            });

            // tty のリサイズ
            self.teletype_manager.resize(*tty_id, width, height);

            let lines = height as u16 / 16;
            let columns = width as u16 / 16;
            self.sender
                .as_mut()
                .unwrap()
                .send(Msg::Resize(WindowSize {
                    num_lines: lines,
                    num_cols: columns,
                    cell_width: 8,
                    cell_height: 8,
                }))
                .unwrap();
        }

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
