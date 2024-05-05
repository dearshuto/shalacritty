use std::hash::Hash;

pub trait IShellManager {
    type Id: Hash + Eq + Copy;
    type Content;

    fn update(&mut self);

    fn spawn(&mut self) -> Self::Id;

    fn send_input(&mut self, id: Self::Id, input: &str);

    fn resize(&mut self, id: Self::Id, width: i32, height: i32);

    fn size(&self, id: Self::Id) -> Option<(u32, u32)>;

    fn is_running(&self, id: Self::Id) -> bool;

    fn consume_dirty(&mut self, id: Self::Id) -> Option<bool>;

    fn enumerate_content(&self, id: Self::Id) -> impl Iterator<Item = Self::Content>;

    fn get_cursor_position(&self, id: Self::Id) -> (u32, u32);
}
