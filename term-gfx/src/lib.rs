// TODO: 隠蔽する
mod backends;
pub mod detail;
mod renderer;
mod traits;

pub use renderer::Renderer;
pub use traits::{IBackend, IContent};
