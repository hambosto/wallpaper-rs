use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use wayland_client::{Connection, EventQueue};

use crate::config::Config;
use crate::globals::BoundGlobals;
use crate::output::ResolvedOutput;
use crate::renderer::ImageRenderer;
use crate::state::WaylandState;

pub fn run(config: &Config) -> Result<()> {
    let mut session = Session::connect()?;
    session.enumerate_outputs()?;
    session.create_surfaces()?;
    session.render(config)?;
    session.event_loop()
}

struct Session {
    state: WaylandState,
    queue: EventQueue<WaylandState>,
    connection: Connection,
    outputs: Vec<ResolvedOutput>,
}

impl Session {
    fn connect() -> Result<Self> {
        tracing::info!("Connecting to Wayland display");

        let connection = Connection::connect_to_env().context("Failed to connect to Wayland display")?;
        let (globals, queue) = wayland_client::globals::registry_queue_init::<WaylandState>(&connection).context("Failed to init globals registry")?;
        let queue_handle = queue.handle();
        let state = BoundGlobals::new(&globals, &queue_handle)?.into_wayland_state(&globals, &queue_handle);

        tracing::info!("Connected");
        Ok(Self { state, queue, connection, outputs: Vec::new() })
    }

    fn enumerate_outputs(&mut self) -> Result<()> {
        self.queue.roundtrip(&mut self.state).context("Roundtrip failed")?;
        self.outputs = ResolvedOutput::resolve_all(&self.state.output_state);

        match self.outputs.len() {
            0 => match self.state.output_state.outputs().count() {
                0 => anyhow::bail!("No wl_output objects found — no displays detected"),
                n => anyhow::bail!("{n} wl_output(s) found but none finished configuring"),
            },
            n => tracing::info!("Outputs resolved: {n}"),
        }

        Ok(())
    }

    fn create_surfaces(&mut self) -> Result<()> {
        let queue_handle = self.queue.handle();
        self.state.create_surfaces(&self.outputs, &queue_handle);
        self.queue.roundtrip(&mut self.state).context("Roundtrip failed")?;
        Ok(())
    }

    fn render(&mut self, config: &Config) -> Result<()> {
        let renderer = ImageRenderer::open(&config.image)?;
        tracing::info!("Rendering wallpapers: {}", config.image.display());

        match self.state.commit_wallpapers(&renderer)? {
            0 => anyhow::bail!("No wallpapers were set — check your configuration"),
            n => tracing::info!("Wallpapers committed: {n}"),
        }

        self.queue.roundtrip(&mut self.state).context("Roundtrip failed")?;
        Ok(())
    }

    fn event_loop(mut self) -> Result<()> {
        tracing::info!("Entering event loop");

        let mut event_loop: EventLoop<WaylandState> = EventLoop::try_new().context("Failed to create event loop")?;
        WaylandSource::new(self.connection, self.queue).insert(event_loop.handle()).context("Failed to insert Wayland source")?;

        event_loop.run(None, &mut self.state, |_| {}).context("Event loop error")?;

        tracing::info!("Exiting");

        Ok(())
    }
}
