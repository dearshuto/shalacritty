use renge::ParametricService;
use warp::Filter;

use crate::services::rendering_service::CaptureRequest;

pub struct ZellijBridgeParams {
    pub api_request_sender: tokio::sync::mpsc::Sender<CaptureRequest>,
}

pub struct ZellijBridgeService {}

impl ZellijBridgeService {
    pub fn new() -> Self {
        Self {}
    }
}

impl ParametricService for ZellijBridgeService {
    type Params = ZellijBridgeParams;

    async fn serve(self, params: Self::Params, _cancellation_token: renge::CancellationToken) {
        println!("SERVE");
        // GET /hello/world のルーティング
        let debug = warp::path!("debug")
            .and(warp::get()) // GETメソッドのみ許可
            .map(|| "Hello, World!");
        let capture = warp::path!("capture").and(warp::get()).and_then(move || {
            let api_request_sender = params.api_request_sender.clone();
            async move {
                let (tx, rx) = tokio::sync::oneshot::channel();
                api_request_sender
                    .send(CaptureRequest { handle: tx })
                    .await
                    .map_err(|_| warp::reject())?;
                // TODO: キャプチャー結果を返す
                let _response = rx.await.map_err(|_| warp::reject())?;
                Ok::<_, warp::Rejection>(warp::reply::reply())
            }
        });
        let event = warp::path("data")
            .and(warp::post())
            .and(warp::body::bytes())
            .map(|_bytes| "POST!");
        warp::serve(debug.or(capture).or(event))
            .run(([127, 0, 0, 1], 5050))
            .await;
    }
}
