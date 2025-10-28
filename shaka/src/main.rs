use shaka::PluginSystem;

#[tokio::main]
async fn main() {
    let _ = asura::Multiplexer::new();

    PluginSystem::new().execute();
}
