mod teletype_manager_ex;
mod vw;

pub use teletype_manager_ex::{TeletypeManagerEx, TerminalAccessor, TerminalProxy};
pub use vw::{VirtualWindowId, VirtualWindowManager};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeletypeId {
    pub(crate) internal: u64,
}
