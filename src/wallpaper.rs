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
    let session = Session::connect()?;
    let with_outputs = session.enumerate_outputs()?;
    let pending = with_outputs.create_surfaces()?;
    let ready = pending.wait_for_configure()?;
    let active = ready.render(config)?;
    active.event_loop()
}

struct Session {
    state: WaylandState,
    queue: EventQueue<WaylandState>,
    connection: Connection,
}

impl Session {
    fn connect() -> Result<Self> {
        tracing::info!("Connecting to Wayland display");

        let conn = Connection::connect_to_env().context("Failed to connect to Wayland display")?;
        let (globals, queue) = wayland_client::globals::registry_queue_init::<WaylandState>(&conn).context("wayland globals registry")?;
        let qh = queue.handle();
        let state = BoundGlobals::bind(&globals, &qh)?.into_wayland_state(&globals, &qh);
        tracing::info!("Connected");

        Ok(Self { state, queue, connection: conn })
    }

    fn enumerate_outputs(mut self) -> Result<WithOutputs> {
        self.queue.roundtrip(&mut self.state).context("Roundtrip failed")?;
        self.queue.roundtrip(&mut self.state).context("Roundtrip failed")?;

        let outputs = ResolvedOutput::resolve_all(&self.state.output_state);

        match outputs.len() {
            0 => match self.state.output_state.outputs().count() {
                0 => anyhow::bail!("No wl_output objects found — no displays detected"),
                n => anyhow::bail!("{n} wl_output(s) found but none finished configuring"),
            },
            n => tracing::info!("Outputs resolved: {n}"),
        }

        Ok(WithOutputs { inner: self, outputs })
    }
}

struct WithOutputs {
    inner: Session,
    outputs: Vec<ResolvedOutput>,
}

impl WithOutputs {
    fn create_surfaces(mut self) -> Result<PendingConfigure> {
        let qh = self.inner.queue.handle();
        let outputs = std::mem::take(&mut self.outputs);

        self.inner.state.create_surfaces(&outputs, &qh);

        Ok(PendingConfigure { inner: self.inner })
    }
}

struct PendingConfigure {
    inner: Session,
}

impl PendingConfigure {
    fn wait_for_configure(mut self) -> Result<ReadyToRender> {
        tracing::info!("Waiting for configure");
        self.inner.queue.roundtrip(&mut self.inner.state).context("Roundtrip failed")?;

        Ok(ReadyToRender { inner: self.inner })
    }
}

struct ReadyToRender {
    inner: Session,
}

impl ReadyToRender {
    fn render(mut self, config: &Config) -> Result<Active> {
        let renderer = ImageRenderer::open(&config.image)?;
        tracing::info!("Rendering wallpapers: {}", config.image.display());

        match self.inner.state.commit_wallpapers(&renderer)? {
            0 => anyhow::bail!("No wallpapers were set — check your configuration"),
            n => tracing::info!("Wallpapers committed: {n}"),
        }

        self.inner.queue.roundtrip(&mut self.inner.state).context("Roundtrip failed")?;

        Ok(Active { state: self.inner.state, queue: self.inner.queue, connection: self.inner.connection })
    }
}

struct Active {
    state: WaylandState,
    queue: EventQueue<WaylandState>,
    connection: Connection,
}

impl Active {
    fn event_loop(mut self) -> Result<()> {
        tracing::info!("Entering event loop");

        ctrlc::set_handler(|| {
            tracing::info!("Exiting");
            std::process::exit(0);
        })?;

        let mut event_loop: EventLoop<WaylandState> = EventLoop::try_new().context("Failed to create event loop")?;
        WaylandSource::new(self.connection, self.queue).insert(event_loop.handle()).context("Failed to insert Wayland source")?;

        event_loop.run(None, &mut self.state, |_| {}).context("Event loop error")?;

        tracing::info!("Exiting");

        Ok(())
    }
}
