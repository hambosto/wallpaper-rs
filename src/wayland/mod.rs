mod dispatch;
mod globals;
mod output;
mod state;
mod surface;

use anyhow::{Context, Result};
use wayland_client::Connection;

use crate::config::Config;
use crate::renderer::ImageRenderer;
use crate::shm::ShmBufferBuilder;

pub use state::WaylandState;

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
        let mut state = WaylandState::default();

        globals::bind_all(&global_list, &qh, &mut state)?;

        event_queue.roundtrip(&mut state).context("Initial roundtrip")?;
        event_queue.roundtrip(&mut state).context("Output-done roundtrip")?;

        let bound = state.globals.as_ref().context("Required Wayland globals not bound")?;
        let (compositor, shm, layer_shell) = (bound.compositor.clone(), bound.shm.clone(), bound.layer_shell.clone());

        let outputs = output::resolve(&state.outputs);
        if outputs.is_empty() {
            let total = state.outputs.len();
            if total > 0 {
                anyhow::bail!("{total} wl_output(s) found but none finished configuring (no wl_output::Done received) — try adding an extra roundtrip");
            }
            anyhow::bail!("No wl_output objects found");
        }

        surface::create_for_outputs(&compositor, &layer_shell, &outputs, &mut state, &qh);
        event_queue.roundtrip(&mut state).context("Configure roundtrip")?;

        let renderer = ImageRenderer::open(&self.config.image)?;
        let count = commit_wallpapers(&shm, &mut state, &renderer, &qh)?;

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

fn commit_wallpapers(shm: &wayland_client::protocol::wl_shm::WlShm, state: &mut WaylandState, renderer: &ImageRenderer, qh: &wayland_client::QueueHandle<WaylandState>) -> Result<usize> {
    let pending = std::mem::take(&mut state.pending);
    let mut count = 0;

    for ps in pending {
        let Some(serial) = ps.configure_serial else {
            println!("warning: no configure received for '{}', skipping", ps.output_name);
            continue;
        };

        ps.layer_surface.ack_configure(serial);

        let (w, h) = (ps.width, ps.height);
        let buffer = ShmBufferBuilder::new(shm, w, h, qh)
            .build_with(|dst| renderer.render_into(dst, w, h))
            .with_context(|| format!("Render wallpaper for '{}'", ps.output_name))?;

        ps.surface.attach(Some(&buffer.buffer), 0, 0);
        ps.surface.damage_buffer(0, 0, w as i32, h as i32);
        ps.surface.commit();

        state.buffers.push(buffer);

        println!("wallpaper set for '{}'", ps.output_name);
        count += 1;
    }

    Ok(count)
}
