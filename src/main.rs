use crate::{
    backends::{Backend, CloudBackend, GitHubBackend, LocalBackend},
    config::{Config, Plugin},
};
use anyhow::Context;
use fs_err::tokio as fs;
use log::{LevelFilter, info, warn};
use roblox_install::RobloxStudio;
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

mod backends;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .format_timestamp(None)
        .init();

    let config = Config::read().await.context("Failed to read config")?;

    let cwd_path = env::current_dir().unwrap();
    let cwd = cwd_path.file_name().unwrap().to_str().unwrap();

    let studio_paths = RobloxStudio::locate()?;
    let plugins_path = studio_paths.plugins_path().to_path_buf();

    let mut existing_plugins = get_existing_hashes(&plugins_path).await?;

    fn get_or_create_backend<T: Backend>(backend: &mut Option<T>) -> anyhow::Result<&mut T> {
        if backend.is_none() {
            *backend = Some(T::new()?);
        }
        Ok(backend.as_mut().unwrap())
    }

    let mut local_backend: Option<LocalBackend> = None;
    let mut github_backend: Option<GitHubBackend> = None;
    let mut cloud_backend: Option<CloudBackend> = None;

    for (key, plugin) in config.plugins {
        let backend: &mut dyn Backend = match &plugin {
            Plugin::Local(_) => get_or_create_backend(&mut local_backend)?,
            Plugin::GitHub(_) => get_or_create_backend(&mut github_backend)?,
            Plugin::Cloud(_) => get_or_create_backend(&mut cloud_backend)?,
        };

        let id = backend.plugin_id(&plugin, &key, cwd);
        info!("Reading \"{key}\"...");
        let (data, ext) = backend.download(&plugin).await?;

        let mut path = plugins_path.join(&id);

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
