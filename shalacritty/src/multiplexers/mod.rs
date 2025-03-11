mod shell_manager;
mod shell_service;
mod tile_manager;

pub mod detail;
pub use shell_manager::{IContent, IPosition, IShellManager};
pub use shell_service::ShellService;
pub use tile_manager::{TileId, TileManager};
