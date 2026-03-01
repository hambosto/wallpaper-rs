use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const APP_NAME: &str = "wallpaper-rs";
const CONFIG_FILE: &str = "config.toml";

pub struct Config {
    pub image: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;

        let contents = std::fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;

        let table: toml::Table = toml::from_str(&contents).with_context(|| format!("Failed to parse {}", path.display()))?;

        let raw = table["image"].as_str().context("Missing or invalid `image` key in config")?;

        let image = validate_absolute(raw)?;

        std::fs::metadata(&image).with_context(|| format!("Cannot access image '{}'", image.display()))?;

        Ok(Self { image })
    }
}

fn validate_absolute(p: &str) -> Result<PathBuf> {
    let path = Path::new(p);

    if !path.is_absolute() {
        anyhow::bail!("`image` must be an absolute path, got '{}'", p);
    }

    Ok(path.to_path_buf())
}

fn config_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .context("$HOME and $XDG_CONFIG_HOME are both unset")?;

    Ok(base.join(APP_NAME).join(CONFIG_FILE))
}
