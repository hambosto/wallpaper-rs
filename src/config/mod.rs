mod types;

use std::path::Path;

use anyhow::{Context, Result};
pub(crate) use types::{Config, CropGravity, Position, ResizeConfig, ResizeStrategy, TransitionConfig, TransitionType};
use xdg::BaseDirectories;

const CONFIG_PREFIX: &str = "wallpaper-rs";
const CONFIG_FILE: &str = "config.toml";

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let xdg_dirs = BaseDirectories::with_prefix(CONFIG_PREFIX);
        let config_file = xdg_dirs
            .find_config_file(CONFIG_FILE)
            .with_context(|| format!("no configuration found at ~/.config/{CONFIG_PREFIX}/{CONFIG_FILE}"))?;

        Self::load_from_file(&config_file)
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("cannot read from config file")?;
        let config: Self = toml::from_str(&content).context("cannot parse config file")?;

        tracing::info!(
            image = %config.image.path.display(),
            resize = ?config.resize.strategy,
            crop_gravity = ?config.resize.crop_gravity,
            transition = ?config.transition.transition_type,
            "config loaded"
        );

        Ok(config)
    }
}
