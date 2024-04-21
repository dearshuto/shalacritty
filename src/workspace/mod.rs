mod detail;
mod diff_calculator;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alacritty_terminal::{
    event::WindowSize,
    event_loop::{EventLoopSender, Msg},
    grid::Indexed,
    term::cell::Cell,
};
use winit::{event_loop::EventLoopWindowTarget, window::WindowId};

use crate::{
    gfx::{ContentPlotter, GlyphManager, Renderer, RendererUpdateParams},

    // 本体は detail 以下にはアクセスさせたくない
    // multiplexers モジュールへの移植途中の互換性保持として直接参照している
    multiplexers::{
        detail::{VirtualWindowId, VirtualWindowManager},
        TileId, TileManager,
    },

    tty::{TeletypeId, TeletypeManager},
    window::WindowManager,
    ConfigService,
};

use self::detail::{ConfigDiff, MultiplexersAdapter};

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

    // WindowId -> TileId
    tile_id_set: HashSet<TileId>,

    // 操作対象となっているウィンドウ
    active_window_id: Option<VirtualWindowId>,

    tile_manager: TileManager<MultiplexersAdapter>,

    // 設定の差分
    config_diff: ConfigDiff,
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

        let (tile_manager, tile_id) = TileManager::new(MultiplexersAdapter::new());

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
            tile_id_set: HashSet::from([tile_id]),
            active_window_id: None,
            tile_manager,
            config_diff: ConfigDiff::new(),
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
        // 設定の差分検出
        self.config_diff
            .update(&self.config_service.read().unwrap());

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

        // 将来的にこれに載せ替える
        for window_id in self.window_manager.ids() {
            for tile_id in &self.tile_id_set {
                let Some(window) = self.window_manager.try_get_window(*window_id) else {
                    continue;
                };

                let _contents = self.tile_manager.enumerate_content(*tile_id);
                let _update_params = RendererUpdateParams::<String>::new(
                    window.inner_size().width,
                    window.inner_size().height,
                )
                // .with_diff(diff)
                // .with_background_color(background)
                // .with_image_alpha(image_alpha)
                // .with_image_path(image_path.clone())
                ;
                // self.renderer.update(*window_id, update_params);
            }
        }

        let is_config_dirty = self.config_diff.is_dirty();
        let background = self.config_diff.consume_clear_color();
        let image_path = self.config_diff.consume_background_path_migrated();
        let image_alpha = self.config_diff.consume_image_alpha();

        for window_id in self.window_manager.ids() {
            // 最描画要求
            let Some(window) = self.window_manager.try_get_window(*window_id) else {
                return;
            };

            let Some(tty_ids) = self.window_tty_table.get(window_id) else {
                continue;
            };

            // 差分がなかったらなにもしない
            let is_tty_dirty = tty_ids.iter().all(|id| self.teletype_manager.is_dirty(*id));
            if !is_tty_dirty && !is_config_dirty {
                continue;
            }

            for tty_id in tty_ids {
                // レンダラーに反映
                self.teletype_manager.get_content(*tty_id, |c| {
                    let cells = c.display_iter.collect::<Vec<Indexed<&Cell>>>();
                    let diff = self.content_plotter.calculate_diff(
                        &cells,
                        &c.cursor.point,
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
                self.teletype_manager.clear_dirty(*tty_id);
            }

            window.request_redraw();
        }
    }

    pub fn render(&mut self, id: WindowId) {
        self.renderer.render(id);
    }

    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        // 仮想ウインドウにリサイズを反映
        self.virtual_window_manager.resize(width, height);
        // ↑ は載せ替え予定
        self.tile_manager.resize(width, height);

        self.renderer.resize(id, width, height);

        let Some(tty_ids) = self.window_tty_table.get(&id) else {
            return;
        };

        for tty_id in tty_ids {
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
        // 将来的に乗り換え予定
        // この行以外は不要になる
        self.tile_manager.send_input(text);

        // multiplexers_adapter にコピった実装があるので統一すべし
        let mut bytes = Vec::with_capacity(text.len() + 1);
        bytes.extend_from_slice(text.as_bytes());
        if text.is_empty() {
            bytes.push(b'\x1b');
        }

        let send_data: std::borrow::Cow<[u8]> = match text {
            // 上
            "ArrowUp" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x41]),
            "\u{f700}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x41]),
            // 下
            "ArrowDown" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x42]),
            "\u{f701}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x42]),
            // 左
            "ArrowLeft" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x44]),
            "\u{f702}" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x44]),
            // 右
            "ArrowRight" => std::borrow::Cow::Borrowed(&[0x1b, 0x5b, 0x43]),
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
