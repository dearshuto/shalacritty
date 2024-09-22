// use clap::Parser;
// use shalacritty::App;

// /// Simple program to greet a person
// #[derive(Parser, Debug)]
// #[command(version, about, long_about = None)]
// struct Args {}

// #[tokio::main]
// async fn main() {
//     let _args = Args::parse();

//     App::run().await;
// }

use shalacritty_core::IProfilerSubject;
use shalacritty_macro::profile;

struct Mock;

impl Mock {
    #[profile]
    pub fn func(&mut self) {}
}

impl IProfilerSubject for Mock {
    fn begin(&self, id: &str) {
        println!("Begin: {}", id);
    }

    fn end(&self) {
        println!("End");
    }
}

#[tokio::main]
async fn main() {
    Mock {}.func();
}
