mod detail;
mod graphics_engine;
mod offscreen_target;
pub mod vkutil;

pub use graphics_engine::GraphicsEngine;
pub use offscreen_target::OffscreenTarget;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
