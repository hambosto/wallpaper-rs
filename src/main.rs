use anyhow::{Context, Result};

mod config;
mod dispatch;
mod render;
mod session;
mod shm;
mod state;

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_file(true).with_line_number(true).init();

    let version = option_env!("WALLPAPER_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    tracing::info!("Starting wallpaper-rs {version}");

    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    session::run(&config::Config::load()?)
}
