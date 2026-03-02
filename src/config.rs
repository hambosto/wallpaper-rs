use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

const APP_NAME: &str = "wallpaper-rs";
const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize)]
pub struct Config {
    pub image: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;
        let raw = std::fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;

        let config: Config = toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;

        let image = require_absolute(&config.image)?;
        std::fs::metadata(&image).with_context(|| format!("Cannot access image '{}'", image.display()))?;

        Ok(Self { image })
    }

    fn default_path() -> Result<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .context("$HOME and $XDG_CONFIG_HOME are both unset")?;

        Ok(base.join(APP_NAME).join(CONFIG_FILE))
    }
}

fn require_absolute(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        anyhow::bail!("`image` must be an absolute path, got '{}'", path.display());
    }
    Ok(path.to_path_buf())
}
