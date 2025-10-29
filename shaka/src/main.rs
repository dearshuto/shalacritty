use shaka::App;
use winit::event_loop::EventLoop;

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::builder().build().unwrap();
    event_loop.run_app(&mut App::new()).unwrap();
}
