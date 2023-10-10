use crate::app::App;

mod app;

#[tokio::main]
async fn main() {
    App::run().await;
}
