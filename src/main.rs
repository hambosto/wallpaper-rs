mod config;
mod image;
mod transition;
mod wayland;

use anyhow::Result;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

use crate::config::Config;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_file(true).with_line_number(true).init();

    let version = option_env!("WALLPAPER_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    tracing::info!(version, "starting wallpaper-rs");

    let config = Config::load()?;
    wayland::run(&config)
}
