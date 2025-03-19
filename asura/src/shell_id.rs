use crate::TeletypeId;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShellId {
    ttyi_id: TeletypeId,
}

impl ShellId {
    pub(crate) fn new(internal: TeletypeId) -> Self {
        Self { ttyi_id: internal }
    }
}
