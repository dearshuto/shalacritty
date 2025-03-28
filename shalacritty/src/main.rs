use clap::Parser;
use shalacritty::App;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// プロファイル機能の有効性
    #[arg(long("enable-profile-server"), default_value_t = false)]
    is_profile_server_enabled: bool,
}

fn main() {
    #[cfg(feature = "tokio-subscriber")]
    console_subscriber::init();

    let args = Args::parse();
    let renderer = term_gfx::Renderer::new();

    // tracing が有効なときのみトレーシングを実行するような分岐
    let _instance = if cfg!(feature = "tracing") {
        use tracing_chrome::ChromeLayerBuilder;
        use tracing_subscriber::prelude::*;

        let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
        Some(guard)
    } else {
        None
    };

    App::run(renderer, args.is_profile_server_enabled);
}
