use std::mem;

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
    _layer_surface: LayerSurface,
    _surface: WlSurface,
}

pub struct WaylandState {
    registry_state: RegistryState,
    pub output_state: OutputState,
    compositor_state: CompositorState,
    layer_shell_state: LayerShell,
    shm_state: Shm,
    pending: Vec<PendingSurface>,
    surfaces: Vec<ActiveSurface>,
    buffers: Vec<ShmBuffer>,
}

impl WaylandState {
    pub fn new(registry_state: RegistryState, output_state: OutputState, compositor_state: CompositorState, layer_shell_state: LayerShell, shm_state: Shm) -> Self {
        Self { registry_state, output_state, compositor_state, layer_shell_state, shm_state, pending: Vec::new(), surfaces: Vec::new(), buffers: Vec::new() }
    }

    pub fn compositor(&self) -> &CompositorState {
        &self.compositor_state
    }

    pub fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    pub fn outputs(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    pub fn shm(&mut self) -> &mut Shm {
        &mut self.shm_state
    }

    pub fn pending_surfaces(&mut self) -> &mut [PendingSurface] {
        &mut self.pending
    }

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
        let pending = mem::take(&mut self.pending);
        let mut count = 0;

        for ps in pending {
            let Some(_) = ps.configure_serial else {
                continue;
            };

            let (w, h) = (ps.width, ps.height);
            let buffer = ShmBuffer::new(&self.shm_state, w, h, qh, |dst| renderer.render(w, h, dst)).with_context(|| format!("Failed to render wallpaper for {}", ps.output_name))?;

            ps.surface.attach(Some(buffer.buffer()), 0, 0);
            ps.surface.damage_buffer(0, 0, w as i32, h as i32);
            ps.surface.commit();

            self.surfaces.push(ActiveSurface { _layer_surface: ps.layer_surface, _surface: ps.surface });
            self.buffers.push(buffer);

            log::info!("Wallpaper set: output={}, width={}, height={}", ps.output_name, w, h);
            count += 1;
        }

        Ok(count)
    }
}
