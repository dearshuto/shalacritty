mod teletype_manager_ex;
mod vw;

pub use teletype_manager_ex::{Content, TeletypeManagerEx, TerminalAccessor, TerminalProxy};
pub use vw::{TileId, VirtualWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeletypeId {
    pub(crate) internal: u64,
}
