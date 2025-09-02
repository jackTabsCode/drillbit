use anyhow::{Context, bail};
use serde::Deserialize;

pub struct WebClient {
    cookie: String,
    client: reqwest::Client,
}

impl WebClient {
    pub fn new() -> anyhow::Result<Self> {
        let cookie = rbx_cookie::get().context("Couldn't get Roblox cookie")?;
        let client = reqwest::Client::new();
        Ok(Self { cookie, client })
    }

    pub async fn download_plugin(&self, id: u64) -> anyhow::Result<Vec<u8>> {
        let url = format!("https://assetdelivery.roblox.com/v2/asset?id={id}");
        let res = self
            .client
            .get(&url)
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        if !res.status().is_success() {
            bail!("Request failed with status: {}", res.status());
        }

        let asset: AssetResponse = res.json().await?;
        let download_url = asset
            .locations
            .first()
            .ok_or_else(|| anyhow::anyhow!("No download locations found"))?
            .location
            .clone();

        let file_res = self
            .client
            .get(download_url)
            .header("Cookie", &self.cookie)
            .send()
            .await?;

        if !file_res.status().is_success() {
            bail!("Download failed with status: {}", file_res.status());
        }

        Ok(file_res.bytes().await?.to_vec())
    }
}

#[derive(Deserialize)]
struct Location {
    location: String,
}

#[derive(Deserialize)]
struct AssetResponse {
    locations: Vec<Location>,
}
