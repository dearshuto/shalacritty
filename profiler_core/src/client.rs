use std::time::Duration;

use reqwest::{IntoUrl, Url};

use crate::{ProfileListRequest, ProfileListResponse};

pub struct Profile {
    id: u64,

    name: String,

    tick_span_list: Vec<std::time::Duration>,
}

impl Profile {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn duration(&self) -> &[std::time::Duration] {
        &self.tick_span_list
    }
}

pub struct Client {
    base_url: Url,
}

impl Client {
    pub async fn connect<D>(url: D) -> Result<Self, ()>
    where
        D: IntoUrl,
    {
        let Ok(url) = url.into_url() else {
            return Err(());
        };

        Ok(Self { base_url: url })
    }

    pub async fn request_profile(
        &mut self,
        _request: &ProfileListRequest,
    ) -> Result<Vec<Profile>, ()> {
        let mut url = self.base_url.clone();
        url.set_path("profile");
        // url.set_query(Some("id=0"));
        // url.set_query(Some("id=1"));

        let Ok(response) = reqwest::get(url).await else {
            return Err(());
        };

        let Ok(text) = response.text().await else {
            return Err(());
        };

        let Ok(profile_list_response) = serde_json::from_str::<ProfileListResponse>(&text) else {
            return Err(());
        };

        let result: Vec<_> = profile_list_response
            .profile_list
            .iter()
            .map(|x| Profile {
                id: x.id,
                name: x.name.clone(),
                tick_span_list: x
                    .tick_span_nano_list
                    .iter()
                    .map(|span| Duration::from_nanos(*span))
                    .collect(),
            })
            .collect();
        Ok(result)
    }
}
