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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("wallpaper_rs=info")).init();
    log::info!("Starting wallpaper-rs");

    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    let config = config::Config::load()?;
    wallpaper::run(config)
}
