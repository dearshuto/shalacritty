use std::{collections::HashSet, hash::Hash};

pub trait IShellManager {
    type Id: Hash + Eq + Copy;

    fn spawn(&mut self) -> Self::Id;

    fn send_input(&mut self, id: Self::Id, input: &str);

    fn resize(&mut self, id: Self::Id, width: i32, height: i32);

    fn is_running(&self, id: Self::Id) -> bool;
}

#[allow(dead_code)]
pub struct TileManager<TShellManager: IShellManager> {
    shell_manager: TShellManager,

    id_set: HashSet<TShellManager::Id>,

    active_shell_id: Option<TShellManager::Id>,
}

impl<TShellManager: IShellManager> TileManager<TShellManager> {
    #[allow(dead_code)]
    pub fn new(shell_manager: TShellManager) -> Self {
        Self {
            shell_manager,
            id_set: HashSet::default(),
            active_shell_id: None,
        }
    }

    #[allow(dead_code)]
    pub fn update(&mut self) {
        // まだ動いてるやつだけ残す
        self.id_set.retain(|id| self.shell_manager.is_running(*id));
    }

    #[allow(dead_code)]
    pub fn spawn(&mut self) {
        let id = self.shell_manager.spawn();
        self.id_set.insert(id);
    }

    #[allow(dead_code)]
    pub fn send_input(&mut self, input: &str) {
        let Some(active_shell_id) = &self.active_shell_id else {
            return;
        };

        self.shell_manager.send_input(*active_shell_id, input);
    }
}
