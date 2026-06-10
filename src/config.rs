use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct Config {
    pub(crate) image: PathBuf,
}

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let path = Self::path()?;
        tracing::info!("Loading config: {}", path.display());

        let raw = std::fs::read_to_string(&path).context("cannot read config file")?;
        let config: Self = toml::from_str(&raw).context("cannot parse config file")?;
        config.validate()?;

        Ok(config)
    }

    fn path() -> Result<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .context("neither $XDG_CONFIG_HOME nor $HOME is set")
            .map(|base| base.join("wallpaper-rs").join("config.toml"))
    }

    fn validate(&self) -> Result<()> {
        if !self.image.is_absolute() {
            anyhow::bail!("image must be absolute, got {}", self.image.display());
        }

        let meta = std::fs::metadata(&self.image).context("cannot access image")?;
        if !meta.is_file() {
            anyhow::bail!("{} is not a file", self.image.display());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_relative_path() {
        let config = Config { image: PathBuf::from("relative/wallpaper.png") };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_nonexistent_file() {
        let config = Config { image: PathBuf::from("/nonexistent/wallpaper.png") };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_accepts_existing_file() {
        let config = Config { image: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml") };
        assert!(config.validate().is_ok());
    }
}
