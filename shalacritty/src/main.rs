use clap::Parser;
use shalacritty::App;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {}

#[tokio::main]
async fn main() {
    let _args = Args::parse();

    App::run().await;
}
