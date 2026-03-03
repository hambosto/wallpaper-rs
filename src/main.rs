use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

mod buffer;
mod config;
mod dispatch;
mod globals;
mod output;
mod renderer;
mod state;
mod wallpaper;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("wallpaper_rs=info".parse()?))
        .init();

    tracing::info!("Starting wallpaper-rs");

    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    let config = config::Config::load()?;

    wallpaper::run(config)
}
