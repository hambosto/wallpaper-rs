use anyhow::{Context, Result};
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use wayland_client::{Connection, EventQueue, QueueHandle};

use crate::config::Config;
use crate::globals::BoundGlobals;
use crate::output::ResolvedOutput;
use crate::renderer::ImageRenderer;
use crate::state::WaylandState;

pub fn run(config: Config) -> Result<()> {
    let session = Session::connect()?;
    let with_outputs = session.enumerate_outputs()?;
    let pending = with_outputs.create_surfaces()?;
    let ready = pending.wait_for_configure()?;
    let active = ready.render(&config)?;
    active.event_loop()
}

struct Session {
    state: WaylandState,
    eq: EventQueue<WaylandState>,
}

impl Session {
    fn connect() -> Result<Self> {
        log::info!("Connecting to Wayland display");

        let conn = Connection::connect_to_env().context("Failed to connect to Wayland display")?;

        let (globals, eq) = wayland_client::globals::registry_queue_init::<WaylandState>(&conn).map_err(|e| anyhow::anyhow!("Failed to initialise registry: {e:?}"))?;

        let qh = eq.handle();
        let BoundGlobals { compositor, shm, layer_shell } = BoundGlobals::bind(&globals, &qh)?;

        let state = WaylandState::new(RegistryState::new(&globals), OutputState::new(&globals, &qh), compositor, layer_shell, shm);

        log::info!("Connected");
        Ok(Self { state, eq })
    }

    fn enumerate_outputs(mut self) -> Result<WithOutputs> {
        let _ = self.eq.roundtrip(&mut self.state).context("Initial roundtrip failed")?;
        let _ = self.eq.roundtrip(&mut self.state).context("Output roundtrip failed")?;

        let outputs = ResolvedOutput::resolve_all(&self.state.output_state);

        match outputs.len() {
            0 => match self.state.output_state.outputs().count() {
                0 => anyhow::bail!("No wl_output objects found — no displays detected"),
                n => anyhow::bail!("{n} wl_output(s) found but none finished configuring"),
            },
            n => log::info!("Outputs resolved: {}", n),
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
        let qh = self.inner.eq.handle();
        let compositor = self.inner.state.compositor().clone();
        self.inner.state.create_surfaces(&compositor, &self.outputs, &qh);
        Ok(PendingConfigure { inner: self.inner, qh })
    }
}

struct PendingConfigure {
    inner: Session,
    qh: QueueHandle<WaylandState>,
}

impl PendingConfigure {
    fn wait_for_configure(mut self) -> Result<ReadyToRender> {
        log::info!("Waiting for configure");
        let _ = self.inner.eq.roundtrip(&mut self.inner.state).context("Configure roundtrip failed")?;
        Ok(ReadyToRender { inner: self.inner, qh: self.qh })
    }
}

struct ReadyToRender {
    inner: Session,
    qh: QueueHandle<WaylandState>,
}

impl ReadyToRender {
    fn render(mut self, config: &Config) -> Result<Active> {
        let renderer = ImageRenderer::open(&config.image)?;
        log::info!("Rendering wallpapers: {}", config.image.display());

        match self.inner.state.commit_wallpapers(&renderer, &self.qh)? {
            0 => anyhow::bail!("No wallpapers were set — check your configuration"),
            n => log::info!("Wallpapers committed: {}", n),
        }

        let _ = self.inner.eq.roundtrip(&mut self.inner.state).context("Final roundtrip failed")?;
        Ok(Active { inner: self.inner })
    }
}

struct Active {
    inner: Session,
}

impl Active {
    fn event_loop(mut self) -> Result<()> {
        log::info!("Entering event loop");
        loop {
            self.inner.eq.blocking_dispatch(&mut self.inner.state).context("Wayland dispatch error")?;
        }
    }
}
