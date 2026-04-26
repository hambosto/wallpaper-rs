use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use wayland_client::{Connection, EventQueue};

use crate::config::Config;
use crate::output::ResolvedOutput;
use crate::state::WaylandState;

pub fn run(config: &Config) -> Result<()> {
    let (mut state, connection, mut queue) = connect()?;

    queue.roundtrip(&mut state).context("Initial roundtrip failed")?;
    let outputs = resolve_outputs(&state)?;

    state.create_surfaces(&outputs, &queue.handle());
    queue.roundtrip(&mut state).context("Surface roundtrip failed")?;

    render_wallpapers(&mut state, &mut queue, config)?;
    event_loop(state, connection, queue)
}

fn connect() -> Result<(WaylandState, Connection, EventQueue<WaylandState>)> {
    tracing::info!("Connecting to Wayland display");

    let connection = Connection::connect_to_env().context("Failed to connect to Wayland display")?;
    let (globals, queue) = wayland_client::globals::registry_queue_init::<WaylandState>(&connection).context("Failed to init globals registry")?;
    let state = crate::globals::bind(&globals, &queue.handle())?;
    tracing::info!("Connected");

    Ok((state, connection, queue))
}

fn resolve_outputs(state: &WaylandState) -> Result<Vec<ResolvedOutput>> {
    let outputs = ResolvedOutput::resolve_all(&state.output_state);
    if outputs.is_empty() {
        match state.output_state.outputs().count() {
            0 => anyhow::bail!("No wl_output objects found — no displays detected"),
            n => anyhow::bail!("{n} wl_output(s) found but none finished configuring"),
        }
    }
    tracing::info!("Outputs resolved: {}", outputs.len());

    Ok(outputs)
}

fn render_wallpapers(state: &mut WaylandState, queue: &mut EventQueue<WaylandState>, config: &Config) -> Result<()> {
    tracing::info!("Rendering wallpapers: {}", config.image.display());

    let count = state.commit_wallpapers(&config.image)?;
    anyhow::ensure!(count > 0, "No wallpapers were set — check your configuration");
    tracing::info!("Wallpapers committed: {count}");

    queue.roundtrip(state).context("Commit roundtrip failed")?;

    Ok(())
}

fn event_loop(mut state: WaylandState, connection: Connection, queue: EventQueue<WaylandState>) -> Result<()> {
    tracing::info!("Entering event loop");

    let mut event_loop: EventLoop<WaylandState> = EventLoop::try_new().context("Failed to create event loop")?;
    WaylandSource::new(connection, queue).insert(event_loop.handle()).context("Failed to insert Wayland source")?;

    event_loop.run(None, &mut state, |_| {}).context("Event loop error")?;
    tracing::info!("Exiting");

    Ok(())
}
