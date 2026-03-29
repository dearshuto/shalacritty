use renge::Service;
use warp::Filter;

pub struct ZellijBridgeService;

impl ZellijBridgeService {
    pub fn new() -> Self {
        Self
    }
}

impl Service for ZellijBridgeService {
    async fn serve(self, _cancellation_token: renge::CancellationToken) {
        println!("SERVE");
        // GET /hello/world のルーティング
        let debug = warp::path!("debug")
            .and(warp::get()) // GETメソッドのみ許可
            .map(|| "Hello, World!");
        let event = warp::path("data")
            .and(warp::post())
            .and(warp::body::bytes())
            .map(|_bytes| "POST!");
        warp::serve(debug.or(event))
            .run(([127, 0, 0, 1], 5050))
            .await;
    }
}
