use anyhow::{Context, Result};

mod buffer;
mod config;
mod output;
mod renderer;
mod surface;
mod wayland;

fn main() -> Result<()> {
    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;
    let config = config::Config::load()?;
    wayland::run(&config)
}
