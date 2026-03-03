use anyhow::{Context, Result};
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use wayland_client::Connection;

use crate::config::Config;
use crate::renderer::ImageRenderer;
use crate::state::WaylandState;
use crate::{globals, output};

pub struct WallpaperApp {
    config: Config,
}

impl WallpaperApp {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn run(self) -> Result<()> {
        let conn = Connection::connect_to_env().context("Failed to connect to Wayland display")?;

        let (global_list, mut event_queue) = wayland_client::globals::registry_queue_init::<WaylandState>(&conn).map_err(|e| anyhow::anyhow!("{e:?}"))?;

        let qh = event_queue.handle();

        let registry_state = RegistryState::new(&global_list);
        let output_state = OutputState::new(&global_list, &qh);

        let bound = globals::bind_globals(&global_list, &qh)?;

        let compositor_state = bound.compositor;
        let layer_shell_state = bound.layer_shell;
        let shm_state = bound.shm;

        let mut state = WaylandState { registry_state, output_state, compositor_state, layer_shell_state, shm_state, pending: Vec::new(), surfaces: Vec::new(), buffers: Vec::new() };

        event_queue.roundtrip(&mut state).context("Initial roundtrip")?;
        event_queue.roundtrip(&mut state).context("Output roundtrip")?;

        let outputs = output::resolve(&state.output_state);
        if outputs.is_empty() {
            let total = state.output_state.outputs().count();
            if total > 0 {
                anyhow::bail!("{total} wl_output(s) found but none finished configuring");
            }
            anyhow::bail!("No wl_output objects found");
        }

        let compositor = state.compositor_state.clone();
        state.create_surfaces(&compositor, &outputs, &qh);
        event_queue.roundtrip(&mut state).context("Configure roundtrip")?;

        let renderer = ImageRenderer::open(&self.config.image)?;
        let count = state.commit_wallpapers(&renderer, &qh)?;

        if count == 0 {
            anyhow::bail!("No wallpapers were set — check your config");
        }

        event_queue.roundtrip(&mut state).context("Final roundtrip")?;
        println!("Done — {count} output(s) wallpapered");

        loop {
            event_queue.blocking_dispatch(&mut state).context("Wayland dispatch error")?;
        }
    }
}
