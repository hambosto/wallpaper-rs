use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const APP_NAME: &str = "wallpaper-rs";
const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize, Debug)]
pub struct Config {
    pub image: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        tracing::info!("Loading configuration: {}", path.display());

        let raw = std::fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
        let config: Self = toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;
        let image = validated_image_path(&config.image)?;

        tracing::info!("Configuration loaded: {}", image.display());

        Ok(Self { image })
    }
}

fn config_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .context("$HOME and $XDG_CONFIG_HOME are both unset")?;

    Ok(base.join(APP_NAME).join(CONFIG_FILE))
}

fn validated_image_path(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        anyhow::bail!("`image` must be an absolute path, got '{}'", path.display());
    }

    let metadata = std::fs::metadata(path).with_context(|| format!("Cannot access image {}", path.display()))?;
    if !metadata.is_file() {
        anyhow::bail!("Image path '{}' is not a file", path.display());
    }

    Ok(path.to_path_buf())
}
