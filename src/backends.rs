use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use fs_err::tokio as fs;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::{env, process::Command};

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
    token: Option<String>,
}

impl GitHubBackend {
    fn get_token() -> Option<String> {
        let token = env::var("GITHUB_TOKEN")
            .or_else(|_| env::var("GH_TOKEN"))
            .ok()
            .or_else(|| {
                let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
                output
                    .status
                    .success()
                    .then(|| String::from_utf8_lossy(&output.stdout).to_string())
            })?;

        let token = token.trim().to_string();
        (!token.is_empty()).then_some(token)
    }

    async fn download_with_token(
        &self,
        token: &str,
        owner_repo: &str,
        tag: &str,
        asset_name: &str,
    ) -> Result<Vec<u8>> {
        let res = self
            .client
            .get(format!(
                "https://api.github.com/repos/{owner_repo}/releases/tags/{tag}"
            ))
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        if !res.status().is_success() {
            bail!(
                "GitHub release lookup for '{owner_repo}@{tag}' failed with status: {}",
                res.status()
            );
        }

        let release: ReleaseResponse = res.json().await?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .with_context(|| format!("Asset '{asset_name}' not found in release '{tag}'"))?;

        let res = self
            .client
            .get(format!(
                "https://api.github.com/repos/{owner_repo}/releases/assets/{}",
                asset.id
            ))
            .bearer_auth(token)
            .header("Accept", "application/octet-stream")
            .send()
            .await?;

        if !res.status().is_success() {
            bail!(
                "GitHub release download failed with status: {}",
                res.status()
            );
        }

        Ok(res.bytes().await?.to_vec())
    }
}

fn parse_release_url(url: &str) -> Option<(&str, &str, &str)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let (rest, asset_name) = rest.rsplit_once('/')?;
    let (owner_repo, tag) = rest.split_once("/releases/download/")?;
    Some((owner_repo, tag, asset_name))
}

#[async_trait]
impl Backend for GitHubBackend {
    fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder().user_agent("drillbit").build()?,
            token: Self::get_token(),
        })
    }

    async fn download(&mut self, plugin: &Plugin) -> Result<(Vec<u8>, Option<String>)> {
        let Plugin::GitHub(url) = plugin else {
            unimplemented!()
        };

        let ext = url.split('.').next_back().map(|s| s.to_string());

        if let Some(token) = &self.token
            && let Some((owner_repo, tag, asset_name)) = parse_release_url(url)
        {
            let data = self
                .download_with_token(token, owner_repo, tag, asset_name)
                .await?;
            return Ok((data, ext));
        }

        let res = self.client.get(url).send().await?;

        if !res.status().is_success() {
            if self.token.is_none() && res.status() == StatusCode::NOT_FOUND {
                bail!(
                    "GitHub release download failed with status: {}. If this is a private repository, set GITHUB_TOKEN or run `gh auth login`.",
                    res.status()
                );
            }

            bail!(
                "GitHub release download failed with status: {}",
                res.status()
            );
        }

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

#[derive(Deserialize)]
struct ReleaseAsset {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    assets: Vec<ReleaseAsset>,
}
