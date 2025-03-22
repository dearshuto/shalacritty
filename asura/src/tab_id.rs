#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct TabId {
    internal: uuid::Uuid,
}

impl TabId {
    pub(crate) fn new() -> Self {
        Self {
            internal: uuid::Uuid::new_v4(),
        }
    }
}
