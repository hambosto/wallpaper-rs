mod handlers;
mod state;

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use state::WaylandState;
use wayland_client::Connection;

use crate::config::Config;

pub fn run(config: &Config) -> Result<()> {
    let connection = Connection::connect_to_env().context("failed to connect to wayland display")?;
    let (global_list, mut event_queue) = wayland_client::globals::registry_queue_init(&connection).context("failed to initialise globals registry")?;
    let mut state = WaylandState::bind(&global_list, &event_queue.handle())?;

    event_queue.roundtrip(&mut state).context("roundtrip failed")?;

    let mut event_loop = EventLoop::try_new().context("failed to create event loop")?;
    let source = WaylandSource::new(connection, event_queue);
    source.insert(event_loop.handle()).context("failed to insert wayland_source")?;

    state.apply_wallpaper(config, &event_loop.handle())?;

    event_loop.run(None, &mut state, |_| {}).context("event loop error")
}
