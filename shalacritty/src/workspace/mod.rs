mod detail;
mod diff_calculator;

use std::{collections::HashSet, sync::Arc};

use alacritty_terminal::index::{Column, Line, Point};
use copypasta::{ClipboardContext, ClipboardProvider};
use detail::{BackgroundRenderer, IBackgroundRendererContext, ImageCache, ImageId};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tokio::runtime::Runtime;
use winit::{keyboard::ModifiersState, window::WindowId};

use crate::{
    gfx::{
        ContentPlotter, GlyphManager, GlyphTexturePatch, IContent, Renderer, RendererUpdateParams,
    },
    multiplexers::{TileId, TileManager},
    Config,
};

use self::detail::{Action, ConfigDiff, ContentAdapter, MultiplexersAdapter};

pub trait IWorkspaceCallback {
    fn begin(&mut self, time: std::time::SystemTime, id: &str);

    fn end(&mut self, time: std::time::SystemTime, id: &str);
}

pub struct Workspace<'a, TCallback: IWorkspaceCallback> {
    instance: wgpu::Instance,
    config_receiver: tokio::sync::watch::Receiver<Config>,
    glyph_manager: GlyphManager,
    content_plotter: ContentPlotter,

    renderer: Renderer<'a, BackgroundRenderer<BackgroundRendererContext>>,

    // WindowId -> TileId
    tile_id_set: HashSet<TileId>,

    tile_manager: TileManager<MultiplexersAdapter>,

    // 設定の差分
    config_diff: ConfigDiff,

    is_force_dirty: bool,

    background_renderer_context: BackgroundRendererContext,

    image_ids: Vec<ImageId>,

    clipboard_context: ClipboardContext,

    callback: TCallback,
}

impl<'a, TCallback: IWorkspaceCallback> Workspace<'a, TCallback> {
    pub fn new_with_callback(
        runtime: Arc<Runtime>,
        config_receiver: tokio::sync::watch::Receiver<Config>,
        callback: TCallback,
    ) -> Self {
        let clipboard_context = ClipboardContext::new().unwrap();

        let instance = wgpu::Instance::default();
        let glyph_manager = GlyphManager::new(config_receiver.borrow().font_size);
        let content_plotter = ContentPlotter::new();

        let (tile_manager, tile_id) = TileManager::new(MultiplexersAdapter::new());
        let mut image_cache = ImageCache::new(runtime);
        let mut image_ids = Vec::default();
        for path in &config_receiver.borrow().background.path {
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

        let image_alpha = { config_receiver.borrow().image_alpha };
        Self {
            instance,
            config_receiver,
            glyph_manager,
            content_plotter,
            renderer: Renderer::new_with_plugin(BackgroundRenderer::new()),
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
                image_alpha,
                image_cache: Arc::new(image_cache),
                window_size: (640, 480),
            },
            image_ids,
            clipboard_context,
            callback,
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

    pub fn update(&mut self, id: WindowId, width: u32, height: u32) {
        // 設定の差分検出
        let current_config = self.config_receiver.borrow();
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
    }

    pub fn render(&mut self, id: WindowId) {
        // self.renderer.render(id);
        self.renderer.render(id);
    }

    pub fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        self.background_renderer_context.window_size = (width, height);

        self.tile_manager.resize(width, height);

        self.renderer.resize(id, width, height);
    }

    pub fn send_input(&mut self, _id: WindowId, text: &str, modifier_state: ModifiersState) {
        let action = Self::detect_action(text, modifier_state);
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

    pub fn is_empty(&self) -> bool {
        self.tile_manager.is_empty()
    }

    #[allow(dead_code)]
    // pub fn listen_config_changed(&mut self) -> std::sync::mpsc::Receiver<Config> {
    //     self.config_service.listen()
    // }

    fn detect_action(input: &str, modifier_state: ModifiersState) -> Action {
        if modifier_state.contains(ModifiersState::CONTROL)
            && modifier_state.contains(ModifiersState::ALT)
        {
            return Action::DumpDebugInfo;
        }

        // ペースト
        if (modifier_state.contains(ModifiersState::CONTROL)
            || modifier_state.contains(ModifiersState::SUPER))
            && modifier_state.contains(ModifiersState::SHIFT)
            && input == "v"
        {
            return Action::Paste;
        }

        // Ctrl+<1~4>
        for tab_number in 1..=4 {
            if modifier_state.contains(ModifiersState::CONTROL) && input == tab_number.to_string() {
                // インデックスとしては 0 始まりなので -1 しておく
                return Action::ActivateTab(tab_number - 1);
            }
        }

        //===============================================================
        // バイト表現では制御文字と区別できない入力は装飾キーの存在をチェックする

        // Ctrl+n で操作対象のシェルを変更
        if modifier_state.contains(ModifiersState::CONTROL)
            && input == String::from_utf8(vec![14]).unwrap()
        {
            return Action::ActivateNextTile;
        }

        // Ctrl+h で画面分割
        // バイト表現では Backspace の制御文字と区別できないので装飾キーの存在をチェックする
        if modifier_state.contains(ModifiersState::CONTROL)
            && input == String::from_utf8(vec![8]).unwrap()
        {
            return Action::SplitHorizontal;
        }

        // Ctrl+h で画面分割
        // TODO: これも装飾文字の存在をチェックした方がよい
        if input == String::from_utf8(vec![20]).unwrap() {
            return Action::NewTab;
        }
        //===============================================================

        return Action::Input(input);
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
