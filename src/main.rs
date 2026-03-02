use anyhow::{Context, Result};

mod config;
mod renderer;
mod shm;
mod wayland;

fn main() -> Result<()> {
    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    let config = config::Config::load()?;

    wayland::WallpaperApp::new(config).run()
}
