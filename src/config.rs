use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub image: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        tracing::info!("Loading config: {}", path.display());

        let raw = std::fs::read_to_string(&path).context("Cannot read config file")?;
        let config: Self = toml::from_str(&raw).context("Cannot parse config file")?;
        config.validate()?;

        Ok(config)
    }

    fn path() -> Result<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .context("Neither $XDG_CONFIG_HOME nor $HOME is set")
            .map(|base| base.join("wallpaper-rs").join("config.toml"))
    }

    fn validate(&self) -> Result<()> {
        if !self.image.is_absolute() {
            anyhow::bail!("image must be absolute, got {}", self.image.display());
        }

        let meta = std::fs::metadata(&self.image).context("Cannot access image")?;
        if !meta.is_file() {
            anyhow::bail!("{} is not a file", self.image.display());
        }

        Ok(())
    }
}
