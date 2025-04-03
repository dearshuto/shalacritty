mod detail;
mod diff_calculator;

use std::{collections::HashSet, sync::Arc};

use alacritty_terminal::index::{Column, Line, Point};
use copypasta::{ClipboardContext, ClipboardProvider};
use detail::{BackgroundRenderer, IBackgroundRendererContext, ImageCache, ImageId};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tokio::runtime::Runtime;
use tracing::instrument;
use winit::{event_loop::EventLoopProxy, keyboard::ModifiersState, window::WindowId};

use crate::{
    app::UserEvent,
    gfx::{ContentPlotter, GlyphManager, GlyphTexturePatch, Renderer, RendererUpdateParams},
    multiplexers::{TileId, TileManager},
    Config,
};

use self::detail::{ConfigDiff, ContentAdapter, MultiplexersAdapter};
pub use detail::Action;

pub trait IWorkspaceCallback {
    fn begin(&mut self, time: std::time::SystemTime, id: &str);

    fn end(&mut self, time: std::time::SystemTime, id: &str);
}

pub struct Workspace<'a, TCallback: IWorkspaceCallback> {
    instance: wgpu::Instance,
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    diff_receiver: tokio::sync::mpsc::Receiver<crate::detail::Diff>,
    string_receiver: tokio::sync::mpsc::Receiver<String>,
    glyph_patch_receiver: tokio::sync::mpsc::Receiver<Vec<GlyphTexturePatch>>,
    glyph_manager: GlyphManager,
    content_plotter: ContentPlotter,

    renderer: Renderer<'a, BackgroundRenderer<BackgroundRendererContext>>,

    // WindowId -> TileId
    tile_id_set: HashSet<TileId>,

    tile_manager: TileManager<MultiplexersAdapter>,

    // 設定の差分
    config_diff: ConfigDiff,
    config_cache: Config,

    is_force_dirty: bool,

    background_renderer_context: BackgroundRendererContext,

    image_ids: Vec<ImageId>,

    clipboard_context: ClipboardContext,

    callback: TCallback,

    event_loop_proxy: EventLoopProxy<UserEvent>,
}

impl<'a, TCallback: IWorkspaceCallback> Workspace<'a, TCallback> {
    pub fn new_with_callback(
        runtime: Arc<Runtime>,
        mut config_receiver: tokio::sync::mpsc::Receiver<Config>,
        diff_receiver: tokio::sync::mpsc::Receiver<crate::detail::Diff>,
        string_receiver: tokio::sync::mpsc::Receiver<String>,
        glyph_patch_receiver: tokio::sync::mpsc::Receiver<Vec<GlyphTexturePatch>>,
        event_loop_proxy: EventLoopProxy<UserEvent>,
        callback: TCallback,
    ) -> Self {
        let clipboard_context = ClipboardContext::new().unwrap();

        // まずは初期設定を確保
        let config = config_receiver.blocking_recv().unwrap();

        let instance = wgpu::Instance::default();
        let glyph_manager = GlyphManager::new(config.font_size);
        let content_plotter = ContentPlotter::new();

        let (tile_manager, tile_id) = TileManager::new(MultiplexersAdapter::new());
        let mut image_cache = ImageCache::new(runtime);
        let mut image_ids = Vec::default();
        for path in &config.background.path {
            // 監視開始
            let Some(id) = image_cache.register(path) else {
                continue;
            };

            image_ids.push(id);
        }

        // 最初に表示する画像はロード完了を待つ
        if let Some(id) = image_ids.first() {
            image_cache.operate_image_and_wait(*id, |_| {});
        }

        let image_alpha = { config.image_alpha };
        Self {
            instance,
            config_receiver,
            diff_receiver,
            string_receiver,
            glyph_patch_receiver,
            glyph_manager,
            content_plotter,
            renderer: Renderer::new_with_plugin(BackgroundRenderer::new()),
            tile_id_set: HashSet::from([tile_id]),
            tile_manager,
            config_diff: ConfigDiff::new(),
            config_cache: config,
            is_force_dirty: false,
            background_renderer_context: BackgroundRendererContext {
                active_id: if image_ids.is_empty() {
                    None
                } else {
                    Some(image_ids[0])
                },
                image_alpha,
                image_cache: Arc::new(image_cache),
                window_size: (640, 480),
            },
            image_ids,
            clipboard_context,
            callback,
            event_loop_proxy,
        }
    }

    pub async fn assign_window<'w, TWindow>(
        &mut self,
        id: winit::window::WindowId,
        window: TWindow,
        width: u32,
        height: u32,
    ) where
        TWindow: HasWindowHandle + HasDisplayHandle,
    {
        // 初期サイズ反映
        self.resize(id, width, height);

        // 載せ替え予定
        self.renderer.register(id, &self.instance, window).await;
        self.renderer.resize(id, width, height);
    }

    #[instrument]
    pub fn update(&mut self, id: WindowId, width: u32, height: u32) {
        if let Ok(latest_config) = self.config_receiver.try_recv() {
            self.config_cache = latest_config;
        }

        // 設定の差分検出
        let current_config = &self.config_cache;
        self.callback
            .begin(std::time::SystemTime::now(), "config_diff");
        self.config_diff.update(&current_config);
        self.callback
            .end(std::time::SystemTime::now(), "config_diff");

        self.callback
            .begin(std::time::SystemTime::now(), "TileManager::update()");
        self.tile_manager.update();
        self.callback
            .end(std::time::SystemTime::now(), "TileManager::update()");

        // いまのところ使用はしていないが、値を吐き出させないと新たな値を送信できないので処理だけしておく
        let _glyph_texture_patches = if let Ok(glyph_patches) = self.glyph_patch_receiver.try_recv()
        {
            glyph_patches
        } else {
            Vec::default()
        };

        // 冗長だがグリフ抽出のために文字列を別途で取得する
        // チャンネルがつまらないように毎度取得しておく
        let content_string = self.string_receiver.try_recv();

        // 差分検出はこちらに載せ替える予定
        // 例の如くチャンネルがつまらないように値は吐き出させておく
        let _diff = self.diff_receiver.try_recv();

        // シェルがすべて破棄されたらウィンドウを閉じる
        if self.tile_manager.is_empty() {
            self.event_loop_proxy
                .send_event(UserEvent::Exit)
                .unwrap_or_default();
            return;
        }

        let is_config_dirty = self.config_diff.is_dirty();
        self.background_renderer_context.image_alpha = current_config.image_alpha;

        let background = self.config_diff.consume_clear_color();
        let image_path = self.config_diff.consume_background_path_migrated();

        // フォントサイズの更新
        if let Some(font_size) = self.config_diff.consume_font_size() {
            self.glyph_manager.set_font_size(font_size);
        }

        // 強制更新のフラグが立ってたらダーティーフラグは見ない
        if self.is_force_dirty {
            self.is_force_dirty = false;
        } else {
            let Some(is_tty_dirty) = self.tile_manager.consume_dirty() else {
                return;
            };

            // 差分がなかったらなにもしない
            if !is_tty_dirty && !is_config_dirty {
                return;
            }
        }

        self.callback.begin(
            std::time::SystemTime::now(),
            "TileManager::enumerate_content()",
        );
        let contents: Vec<ContentAdapter> = self.tile_manager.enumerate_content().collect();
        self.callback.end(
            std::time::SystemTime::now(),
            "TileManager::enumerate_content()",
        );

        // グリフの抽出
        let glyph_texture_patches: Vec<GlyphTexturePatch> = if let Ok(str) = content_string {
            self.glyph_manager.extract_range(str.chars()).collect()
        } else {
            Vec::default()
        };

        let (cursor_x, cursor_y) = self.tile_manager.get_cursor_position();

        self.callback.begin(
            std::time::SystemTime::now(),
            "ContentPlotter::calculate_diff()",
        );
        let diff = self.content_plotter.calculate_diff(
            contents.into_iter(),
            &Point {
                column: Column::from(cursor_x as usize),
                line: Line::from(cursor_y as usize),
            },
            &self.glyph_manager,
            (width, height),
        );
        self.callback.end(
            std::time::SystemTime::now(),
            "ContentPlotter::calculate_diff()",
        );

        let update_params =
            RendererUpdateParams::new_with_user_data(self.background_renderer_context.clone())
                .with_diff(diff)
                .with_glyph_texture_patches(glyph_texture_patches)
                .with_background_color(background)
                .with_image_path(image_path.clone());
        self.renderer.update_with_user_data(id, &update_params);

        // 再描画要求
        self.event_loop_proxy
            .send_event(UserEvent::RequestRedraw)
            .unwrap_or_default();
    }

    #[instrument]
    pub fn render(&mut self, id: WindowId) {
        // self.renderer.render(id);
        self.renderer.render(id);
    }

    #[instrument]
    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        self.background_renderer_context.window_size = (width, height);

        self.tile_manager.resize(width, height);

        self.renderer.resize(id, width, height);
    }

    #[instrument]
    pub fn send_input(&mut self, _id: WindowId, text: &str, modifier_state: ModifiersState) {
        let action = crate::app::detect_action(text, modifier_state);
        match action {
            Action::Input(str) => self.tile_manager.send_input(str),
            Action::Paste => {
                let Ok(content) = self.clipboard_context.get_contents() else {
                    return;
                };

                self.tile_manager.send_input(&content);
            }
            Action::SplitHorizontal => {
                let id = self.tile_id_set.iter().next().unwrap();
                let new_id = self.tile_manager.split_horizontal(*id);
                self.tile_id_set.insert(new_id);
                self.is_force_dirty = true;

                // for id in self.window_manager.ids() {
                //     let Some(window) = self.window_manager.try_get_window(*id) else {
                //         continue;
                //     };

                //     self.tile_manager
                //         .resize(window.inner_size().width, window.inner_size().height);
                // }
            }
            Action::NewTab => {
                self.is_force_dirty = true;
                self.tile_manager.activate_tab(99)
            }
            Action::ActivateNextTile => {
                self.tile_manager.activate_next_tile();
            }
            Action::ActivateTab(index) => {
                self.is_force_dirty = true;
                self.tile_manager.activate_tab(index);

                // タブが切り替わったタイミングで切り替え先の tty にリサイズをかける
                // ウィンドウのリサイズタイミングで全ての tty をリサイズしてもいいかも
                // MEMO:  ウィンドウはひとつを仮定
                // if let Some(window_id) = self.window_manager.ids().first() {
                //     if let Some(window) = self.window_manager.try_get_window(*window_id) {
                //         self.tile_manager
                //             .resize(window.inner_size().width, window.inner_size().height);
                //     }
                // }

                // 背景画像の切り替え
                let id = self.image_ids.get(index as usize);
                self.background_renderer_context
                    .set_active_image_id(id.cloned());
            }
            Action::DumpDebugInfo => {
                self.tile_manager.dump_for_debug();
            }
        }
    }

    // #[allow(dead_code)]
    // pub fn listen_config_changed(&mut self) -> std::sync::mpsc::Receiver<Config> {
    //     self.config_service.listen()
    // }
}

impl<'a, T: IWorkspaceCallback> std::fmt::Debug for Workspace<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Workspace")?;
        std::fmt::Result::Ok(())
    }
}

/// 背景描画のアダプターとしての実装
#[derive(Clone)]
struct BackgroundRendererContext {
    active_id: Option<ImageId>,

    image_alpha: f32,

    image_cache: Arc<ImageCache>,

    // ひとまずウィンドウはひとつしかないと仮定
    window_size: (u32, u32),
}

impl IBackgroundRendererContext for BackgroundRendererContext {
    fn active_id(&self) -> Option<ImageId> {
        self.active_id
    }

    fn active_image_alpha(&self) -> f32 {
        self.image_alpha
    }

    fn image_cache(&self) -> &ImageCache {
        &self.image_cache
    }

    fn window_size(&self) -> (u32, u32) {
        self.window_size
    }
}

impl BackgroundRendererContext {
    pub fn set_active_image_id(&mut self, id: Option<ImageId>) {
        self.active_id = id;
    }
}

impl IWorkspaceCallback for () {
    fn begin(&mut self, _time: std::time::SystemTime, _id: &str) {}

    fn end(&mut self, _time: std::time::SystemTime, _id: &str) {}
}
