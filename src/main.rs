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
    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    let config = config::Config::load()?;

    wallpaper::WallpaperApp::new(config).run()
}
