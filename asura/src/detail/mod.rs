mod teletype_manager;

pub use teletype_manager::TeletypeManager;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeletypeId {
    pub(crate) internal: u64,
}
