mod detail;
mod diff_calculator;

use std::{collections::HashSet, sync::Arc};

use alacritty_terminal::index::{Column, Line, Point};
use detail::{BackgroundRendererV2, IBackgroundRendererContext, ImageCache, ImageId};
use winit::{event_loop::EventLoopWindowTarget, keyboard::ModifiersState, window::WindowId};

use crate::{
    gfx::{
        ContentPlotter, GlyphManager, GlyphTexturePatch, IContent, Renderer, RendererUpdateParams,
    },
    multiplexers::{TileId, TileManager},
    window::WindowManager,
    ConfigService,
};

use self::detail::{Action, ConfigDiff, ContentAdapter, MultiplexersAdapter};

pub struct Workspace<'a> {
    instance: wgpu::Instance,
    #[allow(dead_code)]
    config_service: Arc<ConfigService>,
    glyph_manager: GlyphManager,
    window_manager: WindowManager,
    content_plotter: ContentPlotter,

    #[allow(dead_code)]
    renderer_v2: Renderer<'a, BackgroundRendererV2<BackgroundRendererContext>>,

    // WindowId -> TileId
    tile_id_set: HashSet<TileId>,

    tile_manager: TileManager<MultiplexersAdapter>,

    // 設定の差分
    config_diff: ConfigDiff,

    is_force_dirty: bool,

    background_renderer_context: BackgroundRendererContext,

    image_ids: Vec<ImageId>,
}

impl<'a> Workspace<'a> {
    pub fn new() -> Self {
        let instance = wgpu::Instance::default();
        let config_service = Arc::new(ConfigService::new());
        let glyph_manager = GlyphManager::new();
        let window_manager = WindowManager::new();
        let content_plotter = ContentPlotter::new();

        let (tile_manager, tile_id) = TileManager::new(MultiplexersAdapter::new());
        let mut image_cache = ImageCache::new();
        let mut image_ids = Vec::default();
        if let Ok(config_service) = config_service.read() {
            for path in &config_service.background.path {
                // 監視開始
                let Some(id) = image_cache.register(path) else {
                    continue;
                };

                image_ids.push(id);
            }
        }

        Self {
            instance,
            config_service,
            glyph_manager,
            window_manager,
            content_plotter,
            renderer_v2: Renderer::new_with_plugin(BackgroundRendererV2::new()),
            tile_id_set: HashSet::from([tile_id]),
            tile_manager,
            config_diff: ConfigDiff::new(),
            is_force_dirty: false,
            background_renderer_context: BackgroundRendererContext {
                active_id: if image_ids.is_empty() {
                    None
                } else {
                    Some(image_ids[0])
                },
                image_cache: Arc::new(image_cache),
                window_size: (640, 480),
            },
            image_ids,
        }
    }

    pub async fn spawn_window<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) {
        let id = self.window_manager.create_window(event_loop).await;
        let window = self.window_manager.try_get_window(id).unwrap();
        let window_size = window.inner_size();

        // 初期サイズ反映
        self.resize(id, window_size.width, window_size.height);

        // 載せ替え予定
        self.renderer_v2.register(id, &self.instance, window).await;
        self.renderer_v2
            .resize(id, window_size.width, window_size.height);
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

            // 強制更新のフラグが立ってたらダーティーフラグは見ない
            if self.is_force_dirty {
                self.is_force_dirty = false;
            } else {
                let Some(is_tty_dirty) = self.tile_manager.consume_dirty() else {
                    continue;
                };

                // 差分がなかったらなにもしない
                if !is_tty_dirty && !is_config_dirty {
                    continue;
                }
            }

            let contents: Vec<ContentAdapter> = self.tile_manager.enumerate_content().collect();

            // グリフの抽出
            let glyph_texture_patches: Vec<GlyphTexturePatch> = self
                .glyph_manager
                .extract_range(
                    contents
                        .iter()
                        .map(|c| c.code())
                        .collect::<Vec<char>>()
                        .into_iter(),
                )
                .collect();

            let (cursor_x, cursor_y) = self.tile_manager.get_cursor_position();
            let diff = self.content_plotter.calculate_diff(
                contents.into_iter(),
                &Point {
                    column: Column::from(cursor_x as usize),
                    line: Line::from(cursor_y as usize),
                },
                &self.glyph_manager,
                (window.inner_size().width, window.inner_size().height),
            );

            let update_params =
                RendererUpdateParams::new_with_user_data(self.background_renderer_context.clone())
                    .with_diff(diff)
                    .with_glyph_texture_patches(glyph_texture_patches)
                    .with_background_color(background)
                    .with_image_alpha(image_alpha)
                    .with_image_path(image_path.clone());
            self.renderer_v2
                .update_with_user_data(*window_id, &update_params);

            window.request_redraw();
        }
    }

    pub fn render(&mut self, id: WindowId) {
        // self.renderer.render(id);
        self.renderer_v2.render(id);
    }

    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        self.background_renderer_context.window_size = (width, height);

        self.tile_manager.resize(width, height);

        self.renderer_v2.resize(id, width, height);

        // 最描画要求
        let Some(window) = self.window_manager.try_get_window(id) else {
            return;
        };
        window.request_redraw();
    }

    pub fn send_input(&mut self, _id: WindowId, text: &str, modifier_state: ModifiersState) {
        let action = Self::detect_action(text, modifier_state);
        match action {
            Action::Input(str) => self.tile_manager.send_input(str),
            Action::SplitHorizontal => {
                let id = self.tile_id_set.iter().next().unwrap();
                let new_id = self.tile_manager.split_horizontal(*id);
                self.tile_id_set.insert(new_id);
            }
            Action::NewTab => {
                self.is_force_dirty = true;
                self.tile_manager.activate_tab(99)
            }
            Action::ActivateTab(index) => {
                self.is_force_dirty = true;
                self.tile_manager.activate_tab(index);

                if let Some(id) = self.image_ids.get(index as usize) {
                    self.background_renderer_context.set_active_image_id(*id);
                };

                let Some(_enhance) = self
                    .config_service
                    .read()
                    .unwrap()
                    .background
                    .enhance
                    .get(index as usize)
                else {
                    return;
                };
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tile_manager.is_empty()
    }

    fn detect_action(input: &str, modifier_state: ModifiersState) -> Action {
        // Ctrl+<0~4>
        for index in 0..5 {
            if modifier_state.contains(ModifiersState::CONTROL)
                && input == String::from_utf8(vec![49 + index]).unwrap()
            {
                return Action::ActivateTab(index as u32);
            }
        }

        // Ctrl+h で画面分割
        if input == String::from_utf8(vec![8]).unwrap() {
            return Action::SplitHorizontal;
        }

        // Ctrl+h で画面分割
        if input == String::from_utf8(vec![20]).unwrap() {
            return Action::NewTab;
        }

        Action::Input(input)
    }
}

/// 背景描画のアダプターとしての実装
#[derive(Clone)]
struct BackgroundRendererContext {
    active_id: Option<ImageId>,

    image_cache: Arc<ImageCache>,

    // ひとまずウィンドウはひとつしかないと仮定
    window_size: (u32, u32),
}

impl IBackgroundRendererContext for BackgroundRendererContext {
    fn active_id(&self) -> Option<ImageId> {
        self.active_id
    }

    fn image_cache(&self) -> &ImageCache {
        &self.image_cache
    }

    fn window_size(&self) -> (u32, u32) {
        self.window_size
    }
}

impl BackgroundRendererContext {
    pub fn set_active_image_id(&mut self, id: ImageId) {
        self.active_id = Some(id);
    }
}
