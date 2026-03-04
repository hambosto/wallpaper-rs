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
    let subscriber = tracing_subscriber::fmt().with_file(true).with_line_number(true).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("Starting wallpaper-rs");

    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    let config = config::Config::load()?;
    wallpaper::run(&config)
}
