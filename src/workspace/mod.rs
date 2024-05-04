mod detail;
mod diff_calculator;

use std::{collections::HashSet, sync::Arc};

use alacritty_terminal::{
    grid::Indexed,
    index::{Column, Line, Point},
    term::cell::Cell,
};
use winit::{event_loop::EventLoopWindowTarget, window::WindowId};

use crate::{
    gfx::{ContentPlotter, GlyphManager, GlyphTexturePatch, Renderer, RendererUpdateParams},
    multiplexers::{TileId, TileManager},
    window::WindowManager,
    ConfigService,
};

use self::detail::{ConfigDiff, MultiplexersAdapter};

pub struct Workspace<'a> {
    instance: wgpu::Instance,
    #[allow(dead_code)]
    config_service: Arc<ConfigService>,
    glyph_manager: GlyphManager,
    window_manager: WindowManager,
    content_plotter: ContentPlotter,
    renderer: Renderer<'a>,

    // WindowId -> TileId
    tile_id_set: HashSet<TileId>,

    tile_manager: TileManager<MultiplexersAdapter>,

    // 設定の差分
    config_diff: ConfigDiff,
}

impl<'a> Workspace<'a> {
    pub fn new() -> Self {
        let instance = wgpu::Instance::default();
        let config_service = Arc::new(ConfigService::new());
        let glyph_manager = GlyphManager::new();
        let window_manager = WindowManager::new();
        let content_plotter = ContentPlotter::new();
        let renderer = Renderer::new();

        let (tile_manager, tile_id) = TileManager::new(MultiplexersAdapter::new());

        Self {
            instance,
            config_service,
            glyph_manager,
            window_manager,
            content_plotter,
            renderer,
            tile_id_set: HashSet::from([tile_id]),
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

        // 初期サイズ反映
        self.resize(id, window_size.width, window_size.height);
    }

    pub fn update(&mut self) {
        // 設定の差分検出
        self.config_diff
            .update(&self.config_service.read().unwrap());

        self.tile_manager.update();

        let is_config_dirty = self.config_diff.is_dirty();
        let background = self.config_diff.consume_clear_color();
        let image_path = self.config_diff.consume_background_path_migrated();
        let image_alpha = self.config_diff.consume_image_alpha();

        for window_id in self.window_manager.ids() {
            // 最描画要求
            let Some(window) = self.window_manager.try_get_window(*window_id) else {
                return;
            };

            for tile_id in &self.tile_id_set {
                // 差分がなかったらなにもしない
                let is_tty_dirty = if let Some(is_dirty) = self.tile_manager.consume_dirty(*tile_id)
                {
                    is_dirty
                } else {
                    false
                };
                if !is_tty_dirty && !is_config_dirty {
                    continue;
                }

                let contents: Vec<Indexed<Cell>> =
                    self.tile_manager.enumerate_content(*tile_id).collect();

                // グリフの抽出
                let glyph_texture_patches: Vec<GlyphTexturePatch> = self
                    .glyph_manager
                    .extract_range(
                        contents
                            .iter()
                            .map(|c| c.c)
                            .collect::<Vec<char>>()
                            .into_iter(),
                    )
                    .collect();

                let (cursor_x, cursor_y) = self.tile_manager.get_cursor_position(*tile_id);
                let diff = self.content_plotter.calculate_diff(
                    contents.into_iter(),
                    &Point {
                        column: Column::from(cursor_x as usize),
                        line: Line::from(cursor_y as usize),
                    },
                    &self.glyph_manager,
                    (window.inner_size().width, window.inner_size().height),
                );
                let update_params = RendererUpdateParams::new(
                    window.inner_size().width,
                    window.inner_size().height,
                )
                .with_diff(diff)
                .with_glyph_texture_patches(glyph_texture_patches)
                .with_background_color(background)
                .with_image_alpha(image_alpha)
                .with_image_path(image_path.clone());
                self.renderer.update(*window_id, update_params);
            }
            // self.tile_manager.clear_dirty();

            window.request_redraw();
        }
    }

    pub fn render(&mut self, id: WindowId) {
        self.renderer.render(id);
    }

    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        self.tile_manager.resize(width, height);

        self.renderer.resize(id, width, height);

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
    }

    pub fn is_empty(&self) -> bool {
        self.tile_manager.is_empty()
    }
}
