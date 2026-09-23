mod config;
mod diff_stream;
mod internal_util;
mod parser_vt100;
mod pty_portable;
mod teletype_manager_ex;
mod vw;

pub use config::ConfigBridge;
pub use diff_stream::{ContentCache, DiffStream};
pub use internal_util::{convert_color_snorm, convert_color_uint, into_snorm};
pub use parser_vt100::ParserVt100;
pub use pty_portable::PtyPortable;
pub use teletype_manager_ex::{Content, TeletypeManagerEx, TerminalAccessor, TerminalProxy};
pub use vw::{TileId, VirtualWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeletypeId {
    pub(crate) internal: u64,
}
