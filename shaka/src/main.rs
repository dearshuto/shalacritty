use shaka::{App, PluginSystem};
use winit::event_loop::EventLoop;

#[tokio::main]
async fn main() {
    let _ = asura::Multiplexer::new();

    PluginSystem::new().execute();

    let event_loop = EventLoop::with_user_event().build().unwrap();
    let event_loop_proxy = event_loop.create_proxy();
    event_loop.run_app(&mut App::new(event_loop_proxy)).unwrap();
}
