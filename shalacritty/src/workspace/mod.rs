mod detail;
mod diff_calculator;

use std::{collections::HashMap, sync::Arc, time::Duration};

use alacritty_terminal::index::{Column, Line, Point};
use copypasta::ClipboardContext;
use detail::{
    AsuraContentAdapter, BackgroundRenderer, IBackgroundRendererContext, ImageCache, ImageId,
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tokio::runtime::Runtime;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tracing::instrument;
use winit::{
    event_loop::EventLoopProxy, keyboard::ModifiersState,
    platform::modifier_supplement::KeyEventExtModifierSupplement, window::WindowId,
};

use crate::{
    app::{KeyboadInputEventArgs, UserEvent, WindowSizeChangedEventArgs},
    gfx::{ContentPlotter, GlyphTexturePatch, Renderer, RendererUpdateParams},
    Config,
};

use self::detail::ConfigDiff;
pub use detail::Action;

pub trait IWorkspaceCallback {
    fn begin(&mut self, time: std::time::SystemTime, id: &str);

    fn end(&mut self, time: std::time::SystemTime, id: &str);
}

pub struct Workspace<'a, TCallback: IWorkspaceCallback> {
    instance: wgpu::Instance,
    input_receiver: tokio::sync::mpsc::Receiver<KeyboadInputEventArgs>,
    window_size_changed_receiver: Option<ReceiverStream<WindowSizeChangedEventArgs>>,
    config_receiver: tokio::sync::mpsc::Receiver<Config>,
    diff_receiver: tokio::sync::mpsc::Receiver<crate::detail::Diff>,
    content_receiver: tokio::sync::mpsc::Receiver<Vec<asura::Content>>,
    cursor_receiver: tokio::sync::mpsc::Receiver<(usize, i32)>,
    glyph_patch_receiver: tokio::sync::mpsc::Receiver<Vec<GlyphTexturePatch>>,
    redraw_requested_receiver: tokio::sync::mpsc::Receiver<WindowId>,
    container: crate::detail::Container,
    content_plotter: ContentPlotter,

    renderer: Renderer<'a, BackgroundRenderer<BackgroundRendererContext>>,

    // 設定の差分
    config_diff: ConfigDiff,
    config_cache: Config,

    is_force_dirty: bool,

    background_renderer_context: BackgroundRendererContext,

    image_ids: Vec<ImageId>,

    #[allow(unused)]
    clipboard_context: ClipboardContext,

    callback: TCallback,

    event_loop_proxy: EventLoopProxy<UserEvent>,

    window_size_table: HashMap<WindowId, (u32, u32)>,

    // 新実装に載せ替えるまでのつなぎで必要
    // 旧実装の差分検出の互換性保持のためにコンテンツ一覧をキャッシュしている
    contents_cache: Vec<AsuraContentAdapter>,
    cursor_cache: (usize, i32),
}

impl<'a, TCallback: IWorkspaceCallback> Workspace<'a, TCallback> {
    pub fn new_with_callback(
        runtime: Arc<Runtime>,
        input_receiver: tokio::sync::mpsc::Receiver<KeyboadInputEventArgs>,
        window_size_changed_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
        mut config_receiver: tokio::sync::mpsc::Receiver<Config>,
        diff_receiver: tokio::sync::mpsc::Receiver<crate::detail::Diff>,
        content_receiver: tokio::sync::mpsc::Receiver<Vec<asura::Content>>,
        cursor_receiver: tokio::sync::mpsc::Receiver<(usize, i32)>,
        glyph_patch_receiver: tokio::sync::mpsc::Receiver<Vec<GlyphTexturePatch>>,
        redraw_requested_receiver: tokio::sync::mpsc::Receiver<WindowId>,
        container: crate::detail::Container,
        event_loop_proxy: EventLoopProxy<UserEvent>,
        callback: TCallback,
    ) -> Self {
        let clipboard_context = ClipboardContext::new().unwrap();

        // まずは初期設定を確保
        let config = config_receiver.blocking_recv().unwrap();

        let instance = wgpu::Instance::default();
        let content_plotter = ContentPlotter::new();

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
            input_receiver,
            window_size_changed_receiver: Some(tokio_stream::wrappers::ReceiverStream::new(
                window_size_changed_receiver,
            )),
            config_receiver,
            diff_receiver,
            content_receiver,
            cursor_receiver,
            glyph_patch_receiver,
            container,
            redraw_requested_receiver,
            content_plotter,
            renderer: Renderer::new_with_plugin(BackgroundRenderer::new()),
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
            window_size_table: HashMap::default(),
            contents_cache: Vec::default(),
            cursor_cache: (0, 0),
        }
    }

    pub async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        let s = self
            .window_size_changed_receiver
            .take()
            .unwrap()
            .throttle(Duration::from_millis(250))
            .fuse();
        futures::pin_mut!(s);

        loop {
            tokio::select!(
            Some(config) = self.config_receiver.recv() => self.apply_config(config),
            Some(args) = self.input_receiver.recv() => self.apply_input(args),
            _ = tokio::time::sleep(Duration::from_millis(20)) => self.update_impl().await,
            Some(args) = s.next() => self.apply_window_size_changed(args),
            Some(id) = self.redraw_requested_receiver.recv() => self.render(id),
            _ = &mut cancellation_token => break,
            );
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

    async fn update_impl(&mut self) {
        let temp = self.window_size_table.clone();

        for (id, (width, height)) in temp {
            self.update(id, width, height).await;
        }
    }

    fn apply_window_size_changed(&mut self, args: WindowSizeChangedEventArgs) {
        self.window_size_table
            .insert(args.id, (args.width, args.height));

        self.resize(args.id, args.width, args.height);
    }

    fn apply_input(&mut self, args: KeyboadInputEventArgs) {
        // 入力の反映
        match args.input_type {
            crate::app::InputType::WindowEvent(key_event) => {
                if let Some(str) = key_event.text_with_all_modifiers() {
                    self.send_input(args.id, str, args.state);
                } else if let Some(str) = key_event.text {
                    self.send_input(args.id, str.as_str(), args.state);
                }
            }
            crate::app::InputType::String(str) => {
                self.send_input(args.id, &str, args.state);
            }
        }
    }

    fn apply_config(&mut self, config: Config) {
        self.config_cache = config;
    }

    #[instrument]
    async fn update(&mut self, id: WindowId, width: u32, height: u32) {
        // 設定の差分検出
        let current_config = &self.config_cache;
        self.callback
            .begin(std::time::SystemTime::now(), "config_diff");
        self.config_diff.update(&current_config);
        self.callback
            .end(std::time::SystemTime::now(), "config_diff");

        // グリフ差分の抽出
        let glyph_texture_patches = if let Ok(glyph_patches) = self.glyph_patch_receiver.try_recv()
        {
            glyph_patches
        } else {
            Vec::default()
        };
        if !glyph_texture_patches.is_empty() {
            self.is_force_dirty = true;
        }

        // 差分検出はこちらに載せ替える予定
        // 例の如くチャンネルがつまらないように値は吐き出させておく
        let _diff = self.diff_receiver.try_recv();

        // コンテンツの取得
        // 将来的にこちらに載せ替える
        if let Ok(contents) = self.content_receiver.try_recv() {
            self.is_force_dirty = true;
            self.contents_cache = contents
                .into_iter()
                .map(Into::<AsuraContentAdapter>::into)
                .collect()
        }

        if let Ok(cursor) = self.cursor_receiver.try_recv() {
            self.is_force_dirty = true;
            self.cursor_cache = cursor;
        }

        let is_config_dirty = self.config_diff.is_dirty();
        self.background_renderer_context.image_alpha = current_config.image_alpha;

        let background = self.config_diff.consume_clear_color();
        let image_path = self.config_diff.consume_background_path_migrated();

        // 強制更新のフラグが立ってたらダーティーフラグは見ない
        if self.is_force_dirty {
            self.is_force_dirty = false;
        } else {
            // 差分がなかったらなにもしない
            if !is_config_dirty {
                return;
            }
        }

        self.callback.begin(
            std::time::SystemTime::now(),
            "ContentPlotter::calculate_diff()",
        );
        let diff = self
            .content_plotter
            .calculate_diff(
                self.contents_cache.iter().cloned(),
                &Point {
                    column: Column::from(self.cursor_cache.0),
                    line: Line::from(self.cursor_cache.1 as usize),
                },
                self.container.clone(),
                (width, height),
            )
            .await;
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
    fn render(&mut self, id: WindowId) {
        // self.renderer.render(id);
        self.renderer.render(id);
    }

    #[instrument]
    fn resize(&mut self, id: WindowId, width: u32, height: u32) {
        self.background_renderer_context.window_size = (width, height);

        self.renderer.resize(id, width, height);
    }

    #[instrument]
    fn send_input(&mut self, _id: WindowId, text: &str, modifier_state: ModifiersState) {
        let action = crate::app::detect_action(text, modifier_state);
        match action {
            Action::Input(_str) => {}
            Action::Paste => {}
            Action::SplitHorizontal => {}
            Action::NewTab => {
                self.is_force_dirty = true;
            }
            Action::ActivateNextTile => {}
            Action::ActivateTab(index) => {
                self.is_force_dirty = true;

                // 背景画像の切り替え
                let id = self.image_ids.get(index as usize);
                self.background_renderer_context
                    .set_active_image_id(id.cloned());
            }
            Action::DumpDebugInfo => {}
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
