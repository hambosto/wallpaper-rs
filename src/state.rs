use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::buffer::ShmBuffer;
use crate::output::ResolvedOutput;

pub struct PendingSurface {
    pub layer_surface: LayerSurface,
    pub surface: WlSurface,
    pub configure_serial: Option<u32>,
    pub output_name: String,
    pub width: u32,
    pub height: u32,
}

pub struct WaylandState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell_state: LayerShell,
    pub shm_state: Shm,
    pub pending: Vec<PendingSurface>,
    _committed: Vec<(LayerSurface, WlSurface, ShmBuffer)>,
}

impl WaylandState {
    pub fn new(registry_state: RegistryState, output_state: OutputState, compositor_state: CompositorState, layer_shell_state: LayerShell, shm_state: Shm) -> Self {
        Self { registry_state, output_state, compositor_state, layer_shell_state, shm_state, pending: Vec::new(), _committed: Vec::new() }
    }

    pub fn create_surfaces(&mut self, outputs: &[ResolvedOutput], qh: &QueueHandle<Self>) {
        for output in outputs {
            let surface = self.compositor_state.create_surface(qh);
            let layer_surface = self
                .layer_shell_state
                .create_layer_surface(qh, surface.clone(), Layer::Background, Some("wallpaper-rs"), Some(&output.handle));

            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            surface.commit();

            self.pending
                .push(PendingSurface { layer_surface, surface, configure_serial: None, output_name: output.name.clone(), width: output.width, height: output.height });
        }
    }

    pub fn commit_wallpapers(&mut self, image_path: &std::path::Path) -> Result<usize> {
        let mut count = 0;
        let mut i = 0;

        while i < self.pending.len() {
            if self.pending[i].configure_serial.is_none() {
                i += 1;
                continue;
            }

            let ps = self.pending.swap_remove(i);
            let buffer = ShmBuffer::new(&self.shm_state, ps.width, ps.height, |dst| {
                if let Err(e) = crate::renderer::render_wallpaper(image_path, ps.width, ps.height, dst) {
                    tracing::error!("Render failed for '{}': {e:#}", ps.output_name);
                }
            })
            .with_context(|| format!("Failed to allocate buffer for {}", ps.output_name))?;

            ps.surface.attach(Some(buffer.wl_buffer()), 0, 0);
            ps.surface.damage_buffer(0, 0, ps.width as i32, ps.height as i32);
            ps.surface.commit();

            tracing::info!(
                output = %ps.output_name,
                width = ps.width,
                height = ps.height,
                "Wallpaper set",
            );

            self._committed.push((ps.layer_surface, ps.surface, buffer));
            count += 1;
        }

        Ok(count)
    }
}
