mod handlers;
mod state;
mod surface;

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use state::State;
use wayland_client::Connection;

use crate::config::Config;

fn setup(connection: Connection) -> Result<(EventLoop<'static, State>, State)> {
    let (global_list, mut event_queue) = wayland_client::globals::registry_queue_init(&connection).context("failed to initialise globals registry")?;
    let mut state = State::bind(&global_list, &event_queue.handle())?;
    event_queue.roundtrip(&mut state).context("roundtrip failed")?;

    let event_loop = EventLoop::try_new().context("failed to create event loop")?;
    let source = WaylandSource::new(connection, event_queue);
    source.insert(event_loop.handle()).context("failed to insert wayland_source")?;

    Ok((event_loop, state))
}

pub(crate) fn run(config: &Config) -> Result<()> {
    let connection = Connection::connect_to_env().context("failed to connect to wayland")?;
    let (mut event_loop, mut state) = setup(connection).context("failed to setup wayland")?;

    state.apply_wallpaper(config, &event_loop.handle())?;
    event_loop.run(None, &mut state, |_| {}).context("event loop error")
}
