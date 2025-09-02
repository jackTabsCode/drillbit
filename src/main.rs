use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use crate::{
    config::{Config, Plugin},
    web::WebClient,
};
use anyhow::Context;
use fs_err::tokio as fs;
use log::{LevelFilter, info, warn};
use roblox_studio_utils::RobloxStudioPaths;

mod config;
mod web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .format_timestamp(None)
        .init();

    let config = Config::read().await.context("Failed to read config")?;

    let paths = RobloxStudioPaths::new().context("Failed to get Roblox Studio paths")?;
    let plugins_path = paths.user_plugins();

    let client = WebClient::new()?;

    let cwd_path = env::current_dir().unwrap();
    let cwd = cwd_path.file_name().unwrap().to_str().unwrap();

    let mut existing_plugins = get_existing_hashes(plugins_path).await?;

    for (key, plugin) in config.plugins {
        let id = plugin_id(&plugin, &key, cwd);
        let mut path = plugins_path.join(&id);

        info!("Reading \"{key}\"...");
        let (data, ext) = read_plugin(&client, &plugin).await?;

        if let Some(ext) = ext {
            path.set_extension(ext);
        }

        let data_hash = blake3::hash(&data);
        if let Some(existing_path) = existing_plugins.get(&data_hash) {
            warn!(
                "\"{key}\" already exists at \"{}\", skipping...",
                existing_path.display()
            );
            continue;
        }

        let display = path.display();

        info!("Writing \"{display}\"...");
        fs::write(&path, &data).await?;
        existing_plugins.insert(data_hash, path);
    }

    info!("Plugins installed successfully!");

    Ok(())
}

async fn get_existing_hashes(
    plugins_path: &Path,
) -> anyhow::Result<HashMap<blake3::Hash, PathBuf>> {
    let mut existing_plugins = HashMap::new();

    let mut entries = fs::read_dir(&plugins_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            let data = fs::read(entry.path()).await?;
            let hash = blake3::hash(&data);
            existing_plugins.insert(hash, entry.path());
        }
    }

    Ok(existing_plugins)
}

fn plugin_id(plugin: &Plugin, key: &str, cwd: &str) -> String {
    match plugin {
        Plugin::Local(path) => {
            let filename = path
                .to_path(".")
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            format!("{}_{}", cwd, filename)
        }
        Plugin::Cloud(id) => {
            format!("{}_{}_{}", cwd, key, id)
        }
    }
}

async fn read_plugin(
    client: &WebClient,
    plugin: &Plugin,
) -> anyhow::Result<(Vec<u8>, Option<String>)> {
    match plugin {
        Plugin::Cloud(id) => {
            let data = client
                .download_plugin(*id)
                .await
                .context("Failed to download plugin")?;

            Ok((data, Some("rbxm".to_string())))
        }
        Plugin::Local(path) => {
            let path = path.to_path(".");
            let data = fs::read(path).await.context("Failed to read plugin")?;

            Ok((data, None))
        }
    }
}
