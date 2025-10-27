use crate::TeletypeId;

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShellId {
    ttyi_id: TeletypeId,
}

impl ShellId {
    pub(crate) fn new(internal: TeletypeId) -> Self {
        Self { ttyi_id: internal }
    }
}
