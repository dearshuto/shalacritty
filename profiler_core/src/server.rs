use std::net::SocketAddr;

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
    pub async fn serve<TAddr>(addr: TAddr)
    where
        TAddr: Into<SocketAddr>,
    {
        let profile = warp::path("profile")
            .and(warp::get())
            // .and(warp::query::<ProfileListRequest>())
            .and_then(Self::get);
        warp::serve(profile).run(addr.into()).await;
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
