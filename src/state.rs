use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_surface::WlSurface;

use super::buffer::{ShmBuffer, ShmBufferBuilder};
use super::output::ResolvedOutput;
use crate::renderer::ImageRenderer;

pub struct PendingSurface {
    pub layer_surface: LayerSurface,
    pub surface: WlSurface,
    pub configure_serial: Option<u32>,
    pub output_name: String,
    pub width: u32,
    pub height: u32,
}

pub struct ActiveSurface {
    pub _layer_surface: LayerSurface,
    pub _surface: WlSurface,
}

pub struct WaylandState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell_state: LayerShell,
    pub shm_state: Shm,
    pub pending: Vec<PendingSurface>,
    pub surfaces: Vec<ActiveSurface>,
    pub buffers: Vec<ShmBuffer>,
}

impl WaylandState {
    pub fn create_surfaces(&mut self, compositor: &CompositorState, outputs: &[ResolvedOutput], qh: &QueueHandle<Self>) {
        for output in outputs {
            let surface = compositor.create_surface(qh);
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

    pub fn commit_wallpapers(&mut self, renderer: &ImageRenderer, qh: &QueueHandle<Self>) -> Result<usize> {
        let pending = std::mem::take(&mut self.pending);
        let mut count = 0;

        for ps in pending {
            let Some(_serial) = ps.configure_serial else {
                println!("warning: no configure received for '{}', skipping", ps.output_name);
                continue;
            };

            let (w, h) = (ps.width, ps.height);
            let buffer = ShmBufferBuilder::new(&self.shm_state, w, h, qh)
                .build_with(|dst| renderer.render_into(dst, w, h))
                .with_context(|| format!("Render wallpaper for '{}'", ps.output_name))?;

            ps.surface.attach(Some(buffer.buffer()), 0, 0);
            ps.surface.damage_buffer(0, 0, w as i32, h as i32);
            ps.surface.commit();

            self.surfaces.push(ActiveSurface { _layer_surface: ps.layer_surface, _surface: ps.surface });
            self.buffers.push(buffer);

            println!("wallpaper set for '{}'", ps.output_name);
            count += 1;
        }

        Ok(count)
    }
}
