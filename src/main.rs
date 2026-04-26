use anyhow::{Context, Result};

mod buffer;
mod config;
mod dispatch;
mod globals;
mod output;
mod renderer;
mod state;
mod wallpaper;

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_file(true).with_line_number(true).init();

    let version = option_env!("WALLPAPER_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    tracing::info!("Starting wallpaper-rs {version}");

    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;
    let config = config::Config::load()?;

    wallpaper::run(&config)
}
