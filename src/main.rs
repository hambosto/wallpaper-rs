mod config;
mod render;
mod transition;
mod wayland;

use anyhow::{Context, Result};
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use xdg::BaseDirectories;

use crate::config::Config;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_file(true).with_line_number(true).init();

    let config = load_config()?;
    let version = option_env!("WALLPAPER_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));

    tracing::info!(version, "starting wallpaper-rs");

    wayland::run(&config)
}

fn load_config() -> Result<Config> {
    let xdg_dirs = BaseDirectories::with_prefix("wallpaper-rs");
    let config_file = xdg_dirs.find_config_file("config.toml").context("no configuration found at ~/.config/wallpaper-rs/config.toml")?;

    Config::new(&config_file)
}
