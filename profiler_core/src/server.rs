use std::net::SocketAddr;

use tokio::sync::oneshot::Receiver;
use warp::{
    reply::{Reply, Response},
    Filter,
};

use crate::{detail::Profile, ProfileListResponse};

pub trait IServerBackend {
    fn count(&self) -> usize;

    fn key(&self, index: usize) -> String;

    fn cache_count(&self, key: &str) -> usize;

    fn duration(&self, key: &str, cache_index: usize) -> std::time::Duration;
}

struct ProfileListReply {
    internal: ProfileListResponse,
}

impl Reply for ProfileListReply {
    fn into_response(self) -> warp::reply::Response {
        let str = serde_json::to_string(&self.internal).unwrap();
        Response::new(str.into())
    }
}

pub struct Server;

impl Server {
    // ([127, 0, 0, 1], 3030)
    pub async fn serve<TAddr, TBackend, T>(addr: TAddr, backend: TBackend, receiver: Receiver<T>)
    where
        TAddr: Into<SocketAddr>,
        TBackend: IServerBackend + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let profile = warp::path("profile")
            .and(warp::get())
            .and(warp::any().map(move || backend.clone()))
            // .and(Self::with_logic(backend.clone()))
            // .and(warp::query::<ProfileListRequest>())
            .and_then(Self::get_with);
        let (_addr, future) =
            warp::serve(profile).bind_with_graceful_shutdown(addr.into(), async move {
                receiver.await.ok();
            });
        future.await;
    }

    async fn get_with<TBackend>(backend: TBackend) -> Result<impl Reply, warp::Rejection>
    where
        TBackend: IServerBackend + Clone + Send + Sync + 'static,
    {
        let profile_list: Vec<_> = (0..backend.count())
            .map(|index| {
                let key = backend.key(index);
                let cache_count = backend.cache_count(&key);
                let tick_span_nano_list: Vec<_> = (0..cache_count)
                    .map(|index| backend.duration(&key, index).as_nanos() as u64)
                    .collect();
                Profile {
                    id: 0,
                    name: key.to_string(),
                    tick_span_nano_list,
                }
            })
            .collect();
        Ok(ProfileListReply {
            internal: ProfileListResponse { profile_list },
        })
    }
}
