pub mod config;
pub mod render;
pub mod wayland;

use anyhow::{Context, Result};

pub fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .init();

    let version = option_env!("WALLPAPER_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    tracing::info!(version, "starting wallpaper-rs");

    std::env::var("WAYLAND_DISPLAY")
        .context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    let config = config::Config::load()?;
    wayland::run(&config)
}
