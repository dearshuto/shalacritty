use std::hash::Hash;

pub trait IShellManager {
    type Id: Hash + Eq + Copy;
    type Content;

    fn spawn(&mut self) -> Self::Id;

    fn send_input(&mut self, id: Self::Id, input: &str);

    fn resize(&mut self, id: Self::Id, width: i32, height: i32);

    fn is_running(&self, id: Self::Id) -> bool;

    fn enumerate_content(&self, id: Self::Id) -> impl Iterator<Item = Self::Content>;
}
