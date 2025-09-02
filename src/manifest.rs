use anyhow::bail;
use fs_err::tokio as fs;
use relative_path::RelativePathBuf;
use serde::Deserialize;
use std::collections::HashMap;

pub const FILE_NAME: &str = "drillbit.toml";
const ALLOWED_EXTS: &[&str] = &["rbxm", "rbxmx", "lua", "luau"];

#[derive(Debug, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub plugins: HashMap<String, Plugin>,
}

impl Manifest {
    pub async fn read() -> anyhow::Result<Manifest> {
        let content = fs::read_to_string(FILE_NAME).await?;
        let manifest: Manifest = toml::from_str(&content)?;

        let allowed_exts = ALLOWED_EXTS.join(", ");

        for (name, plugin) in &manifest.plugins {
            if let Plugin::Local(path) = plugin {
                match path.extension() {
                    Some(extension) if !ALLOWED_EXTS.contains(&extension) => {
                        bail!(
                            "Plugin '{}' has invalid file extension '{}'. Allowed extensions: {}",
                            name,
                            extension,
                            allowed_exts
                        );
                    }
                    None => {
                        bail!(
                            "Plugin '{}' must have a file extension. Allowed extensions: {}",
                            name,
                            allowed_exts
                        );
                    }
                    _ => {}
                }
            }
        }

        Ok(manifest)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plugin {
    Local(RelativePathBuf),
    Cloud(u64),
}
