use std::net::SocketAddr;

use tokio::sync::oneshot::Receiver;
use warp::{
    reply::{Reply, Response},
    Filter,
};

use crate::{ProfileListRequest, ProfileListResponse};

pub struct ProfileListReply {
    internal: ProfileListResponse,
}

impl Reply for ProfileListReply {
    fn into_response(self) -> warp::reply::Response {
        let str = serde_json::to_string(&self.internal).unwrap();
        Response::new(str.into())
    }
}

pub struct Server {}

impl Server {
    // ([127, 0, 0, 1], 3030)
    pub async fn serve<TAddr, T>(addr: TAddr, receiver: Receiver<T>)
    where
        TAddr: Into<SocketAddr>,
        T: Send + Sync + 'static,
    {
        let profile = warp::path("profile")
            .and(warp::get())
            // .and(warp::query::<ProfileListRequest>())
            .and_then(Self::get);
        let (_addr, future) =
            warp::serve(profile).bind_with_graceful_shutdown(addr.into(), async move {
                receiver.await.ok();
            });
        future.await;
    }

    #[allow(dead_code)]
    async fn get_color_impl(_request: ProfileListRequest) -> Result<impl Reply, warp::Rejection> {
        Ok(ProfileListReply {
            internal: Default::default(),
        })
    }

    async fn get() -> Result<impl Reply, warp::Rejection> {
        Ok(ProfileListReply {
            internal: Default::default(),
        })
    }
}
