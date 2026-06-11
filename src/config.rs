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
        let raw = std::fs::read_to_string(&path).context("cannot read config file")?;
        let config: Self = toml::from_str(&raw).context("cannot parse config file")?;
        config.validate()?;
        tracing::info!(image = %config.image.display(), "config loaded");

        Ok(config)
    }

    fn path() -> Result<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .context("neither $XDG_CONFIG_HOME nor $HOME is set")?;

        Ok(base.join("wallpaper-rs").join("config.toml"))
    }

    fn validate(&self) -> Result<()> {
        if !self.image.is_absolute() {
            anyhow::bail!("image path must be absolute, got: {}", self.image.display());
        }

        let meta = std::fs::metadata(&self.image)
            .with_context(|| format!("cannot access image: {}", self.image.display()))?;
        if !meta.is_file() {
            anyhow::bail!("{} is not a regular file", self.image.display());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_relative_path() {
        let config = Config {
            image: PathBuf::from("relative/wallpaper.png"),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_nonexistent_file() {
        let config = Config {
            image: PathBuf::from("/nonexistent/wallpaper.png"),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_accepts_existing_file() {
        let config = Config {
            image: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        };
        assert!(config.validate().is_ok());
    }
}
