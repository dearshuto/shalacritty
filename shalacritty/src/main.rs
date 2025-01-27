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
    let args = Args::parse();
    let renderer = term_gfx::Renderer::new();
    App::run(renderer, args.is_profile_server_enabled);
}
