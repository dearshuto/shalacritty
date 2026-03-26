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
        // GET /hello/world のルーティング
        let hello = warp::path!("hello" / "world")
            .and(warp::get()) // GETメソッドのみ許可
            .map(|| "Hello, World!");

        warp::serve(hello).run(([127, 0, 0, 1], 5050)).await;
    }
}
