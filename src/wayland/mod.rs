mod handlers;
mod state;

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use wayland_client::Connection;

use crate::config::Config;
use state::WaylandState;

pub fn run(config: &Config) -> Result<()> {
    let connection = Connection::connect_to_env().context("failed to connect to wayland display")?;
    let (globals, mut queue) = wayland_client::globals::registry_queue_init::<WaylandState>(&connection).context("failed to initialise globals registry")?;
    let mut state = WaylandState::bind(&globals, &queue.handle())?;

    tracing::info!("connected to wayland display");

    queue.roundtrip(&mut state).context("initial roundtrip failed")?;

    queue.roundtrip(&mut state).context("configure roundtrip failed")?;

    state.set_wallpapers(&config.image.path, &config.transition, &config.resize, &queue.handle())?;

    queue.roundtrip(&mut state).context("commit roundtrip failed")?;

    let mut event_loop = EventLoop::<WaylandState>::try_new().context("failed to create event loop")?;

    WaylandSource::new(connection, queue).insert(event_loop.handle()).context("failed to insert Wayland source")?;

    state.start_animation_timer(&event_loop.handle())?;

    event_loop.run(None, &mut state, |_| {}).context("event loop error")
}
