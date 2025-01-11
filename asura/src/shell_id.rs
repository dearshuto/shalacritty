#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShellId {
    internal: uuid::Uuid,
}

impl ShellId {
    pub fn new() -> Self {
        Self {
            internal: uuid::Uuid::now_v7(),
        }
    }
}
