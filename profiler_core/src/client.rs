use reqwest::{IntoUrl, Url};

use crate::{ProfileListRequest, ProfileListResponse};

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

    pub async fn request_profile(&mut self, _request: &ProfileListRequest) -> Result<Vec<u64>, ()> {
        let mut url = self.base_url.clone();
        url.set_path("profile");
        // url.set_query(Some("id=0"));
        // url.set_query(Some("id=1"));
        println!("{:?}", url.to_string());

        let Ok(response) = reqwest::get(url).await else {
            return Err(());
        };

        let Ok(text) = response.text().await else {
            return Err(());
        };

        let Ok(profile_list_response) = serde_json::from_str::<ProfileListResponse>(&text) else {
            return Err(());
        };

        Ok(profile_list_response.id_list)
    }
}
