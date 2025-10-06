use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use fs_err::tokio as fs;
use reqwest::Client;
use serde::Deserialize;

use crate::config::Plugin;

#[async_trait]
pub trait Backend {
    fn new() -> Result<Self>
    where
        Self: Sized;

    async fn download(&mut self, plugin: &Plugin) -> Result<(Vec<u8>, Option<String>)>;

    fn plugin_id(&self, plugin: &Plugin, key: &str, cwd: &str) -> String;
}

pub struct LocalBackend;

#[async_trait]
impl Backend for LocalBackend {
    fn new() -> Result<Self> {
        Ok(LocalBackend)
    }

    async fn download(&mut self, plugin: &Plugin) -> Result<(Vec<u8>, Option<String>)> {
        let Plugin::Local(path) = plugin else {
            bail!("LocalBackend can only handle Local plugins")
        };

        let mut new_ext = None;

        if let Some(ext) = path.extension()
            && ext == "luau"
        {
            new_ext = Some("lua".to_string());
        }

        let path = path.to_path(".");
        let data = fs::read(path).await.context("Failed to read plugin")?;

        Ok((data, new_ext))
    }

    fn plugin_id(&self, plugin: &Plugin, _key: &str, cwd: &str) -> String {
        let Plugin::Local(path) = plugin else {
            unreachable!()
        };

        let filename = path
            .to_path(".")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        format!("{}_{}", cwd, filename)
    }
}

pub struct CloudBackend {
    cookie: Option<String>,
    client: Client,
}

impl CloudBackend {
    fn get_cookie(&mut self) -> Result<String> {
        if self.cookie.is_none() {
            let cookie = rbx_cookie::get().context("Couldn't get Roblox cookie")?;
            self.cookie = Some(cookie);
        }
        Ok(self.cookie.as_ref().unwrap().clone())
    }
}

#[async_trait]
impl Backend for CloudBackend {
    fn new() -> Result<Self> {
        Ok(Self {
            cookie: None,
            client: Client::new(),
        })
    }

    async fn download(&mut self, plugin: &Plugin) -> Result<(Vec<u8>, Option<String>)> {
        let Plugin::Cloud(id) = plugin else {
            unreachable!()
        };

        let cookie = self.get_cookie()?;
        let url = format!("https://assetdelivery.roblox.com/v2/asset?id={id}");
        let res = self
            .client
            .get(&url)
            .header("Cookie", &cookie)
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
            .header("Cookie", &cookie)
            .send()
            .await?;

        if !file_res.status().is_success() {
            bail!("Download failed with status: {}", file_res.status());
        }

        Ok((file_res.bytes().await?.to_vec(), Some("rbxm".to_string())))
    }

    fn plugin_id(&self, plugin: &Plugin, key: &str, cwd: &str) -> String {
        let Plugin::Cloud(id) = plugin else {
            unreachable!()
        };

        format!("{}_{}_{}", cwd, key, id)
    }
}

pub struct GitHubBackend {
    client: Client,
}

#[async_trait]
impl Backend for GitHubBackend {
    fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
        })
    }

    async fn download(&mut self, plugin: &Plugin) -> Result<(Vec<u8>, Option<String>)> {
        let Plugin::GitHub(url) = plugin else {
            unimplemented!()
        };

        let res = self.client.get(url).send().await?;

        if !res.status().is_success() {
            bail!(
                "GitHub release download failed with status: {}",
                res.status()
            );
        }

        let ext = url.split('.').next_back().map(|s| s.to_string());
        Ok((res.bytes().await?.to_vec(), ext))
    }

    fn plugin_id(&self, plugin: &Plugin, _key: &str, cwd: &str) -> String {
        let Plugin::GitHub(url) = plugin else {
            unimplemented!()
        };

        let filename = url.split('/').next_back().unwrap_or("unknown");
        format!("{}_{}", cwd, filename)
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
