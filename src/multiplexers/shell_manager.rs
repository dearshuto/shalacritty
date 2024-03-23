use std::hash::Hash;

pub trait IShellManager {
    type Id: Hash + Eq + Copy;

    fn spawn(&mut self) -> Self::Id;

    fn send_input(&mut self, id: Self::Id, input: &str);

    fn resize(&mut self, id: Self::Id, width: i32, height: i32);

    fn is_running(&self, id: Self::Id) -> bool;
}
