mod handlers;
mod state;

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use state::WaylandState;
use wayland_client::Connection;

use crate::config::Config;

pub fn run(config: &Config) -> Result<()> {
    std::env::var("WAYLAND_DISPLAY").context("WAYLAND_DISPLAY not set — are you in a Wayland session?")?;

    let connection = Connection::connect_to_env().context("failed to connect to wayland display")?;
    let (global_list, mut event_queue) = wayland_client::globals::registry_queue_init(&connection).context("failed to initialise globals registry")?;
    let queue_handle = event_queue.handle();
    let mut state = WaylandState::bind(&global_list, &queue_handle)?;

    tracing::info!("connected to wayland display");

    event_queue.roundtrip(&mut state).context("initial roundtrip failed")?;
    event_queue.roundtrip(&mut state).context("configure roundtrip failed")?;

    let mut event_loop = EventLoop::try_new().context("failed to create event loop")?;
    WaylandSource::new(connection, event_queue).insert(event_loop.handle()).context("failed to insert Wayland source")?;

    state.apply_wallpaper(config, &event_loop.handle())?;

    event_loop.run(None, &mut state, |_| {}).context("event loop error")
}
