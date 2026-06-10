use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use wayland_client::{Connection, EventQueue};

use crate::config::Config;
use crate::state::WaylandState;

pub(crate) fn run(config: &Config) -> Result<()> {
    let (mut state, connection, mut queue) = connect()?;

    queue.roundtrip(&mut state).context("initial roundtrip failed")?;
    queue.roundtrip(&mut state).context("configure roundtrip failed")?;

    state.set_wallpapers(&config.image)?;
    queue.roundtrip(&mut state).context("commit roundtrip failed")?;

    tracing::info!("Entering event loop");
    let mut event_loop = EventLoop::<WaylandState>::try_new().context("failed to create event loop")?;
    WaylandSource::new(connection, queue).insert(event_loop.handle()).context("failed to insert Wayland source")?;

    event_loop.run(None, &mut state, |_| {}).context("event loop error")
}

fn connect() -> Result<(WaylandState, Connection, EventQueue<WaylandState>)> {
    tracing::info!("Connecting to Wayland display");

    let connection = Connection::connect_to_env().context("failed to connect to Wayland display")?;
    let (globals, queue) = wayland_client::globals::registry_queue_init::<WaylandState>(&connection).context("failed to initialise globals registry")?;
    let state = WaylandState::bind(&globals, &queue.handle())?;

    tracing::info!("Connected");

    Ok((state, connection, queue))
}
