use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tracing::instrument;
use winit::window::WindowId;

use crate::{
    app::WindowSizeChangedEventArgs,
    workspace::{IWorkspaceCallback, Workspace},
};

/// Workspace の更新処理を非同期サービスとして実行するための構造体
/// Workspace の更新処理は将来的にプリミティブな機能として分解する予定なのでこの機能も将来的には廃止予定
pub struct WorkspaceUpdateServiceTentative<T>
where
    T: IWorkspaceCallback,
{
    workspace: Arc<Mutex<Workspace<'static, T>>>,

    polling_event_receiver: tokio::sync::mpsc::Receiver<()>,
    window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,

    window_size_table: HashMap<WindowId, (u32, u32)>,
}

impl<T> WorkspaceUpdateServiceTentative<T>
where
    T: IWorkspaceCallback,
{
    pub fn new(
        workspace: Arc<Mutex<Workspace<'static, T>>>,

        polling_event_receiver: tokio::sync::mpsc::Receiver<()>,
        window_size_receiver: tokio::sync::mpsc::Receiver<WindowSizeChangedEventArgs>,
    ) -> Self {
        Self {
            workspace,
            polling_event_receiver,
            window_size_receiver,
            window_size_table: HashMap::default(),
        }
    }

    #[instrument]
    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(_) = self.polling_event_receiver.recv() => self.update_workspace(),
            Some(args) = self.window_size_receiver.recv() => self.store_window_size(args),
            else => break,
            );
        }
    }

    #[instrument]
    fn update_workspace(&mut self) {
        let Ok(mut workspace) = self.workspace.lock() else {
            return;
        };

        for (id, (width, height)) in &self.window_size_table {
            workspace.update(*id, *width, *height);
        }
    }

    #[instrument]
    fn store_window_size(&mut self, args: WindowSizeChangedEventArgs) {
        self.window_size_table
            .insert(args.id, (args.width, args.height));
    }
}

impl<T: IWorkspaceCallback> std::fmt::Debug for WorkspaceUpdateServiceTentative<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkspaceUpdateServiceTentative")?;
        std::fmt::Result::Ok(())
    }
}
