use std::path::Path;

use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::{OutputInfo, OutputState};
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::shm::ShmBuffer;

pub struct Geometry {
    pub width: u32,
    pub height: u32,
}

pub struct Unconfigured {
    pub layer_surface: LayerSurface,
    pub surface: WlSurface,
    pub geometry: Geometry,
    pub output_name: String,
}

impl Unconfigured {
    pub fn configure(self) -> Configured {
        Configured { layer_surface: self.layer_surface, surface: self.surface, geometry: self.geometry, output_name: self.output_name }
    }
}

pub struct Configured {
    pub layer_surface: LayerSurface,
    pub surface: WlSurface,
    pub geometry: Geometry,
    pub output_name: String,
}

pub struct WaylandState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub compositor: CompositorState,
    pub layer_shell: LayerShell,
    pub shm: Shm,
    pub unconfigured: Vec<Unconfigured>,
    _committed: Vec<(LayerSurface, WlSurface, ShmBuffer)>,
}

impl WaylandState {
    pub fn bind(globals: &GlobalList, qh: &QueueHandle<Self>) -> Result<Self> {
        Ok(Self {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, qh),
            compositor: CompositorState::bind(globals, qh).context("wl_compositor not available")?,
            layer_shell: LayerShell::bind(globals, qh).context("zwlr_layer_shell_v1 not available")?,
            shm: Shm::bind(globals, qh).context("wl_shm not available")?,
            unconfigured: Vec::new(),
            _committed: Vec::new(),
        })
    }

    pub fn create_surfaces(&mut self, qh: &QueueHandle<Self>) {
        let outputs: Vec<_> = self
            .output_state
            .outputs()
            .filter_map(|handle| {
                let info = self.output_state.info(&handle)?;
                let geometry = output_geometry(&info)?;
                let name = info.name.as_deref().map(String::from).unwrap_or_else(|| format!("output-{}", info.id));
                Some((handle, geometry, name))
            })
            .collect();

        for (handle, geometry, name) in outputs {
            let surface = self.compositor.create_surface(qh);
            let layer_surface = self.layer_shell.create_layer_surface(qh, surface.clone(), Layer::Background, Some("wallpaper-rs"), Some(&handle));
            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            surface.commit();
            self.unconfigured.push(Unconfigured { layer_surface, surface, geometry, output_name: name });
        }

        tracing::info!("Surfaces created: {}", self.unconfigured.len());
    }

    pub fn set_wallpapers(&mut self, image: &Path) -> Result<()> {
        let configured: Vec<Configured> = self.unconfigured.drain(..).map(|u| u.configure()).collect();

        if configured.is_empty() {
            anyhow::bail!("No surfaces were configured by the compositor");
        }

        for c in configured {
            let buffer = ShmBuffer::allocate_and_fill(&self.shm, c.geometry.width, c.geometry.height, |dst| crate::render::render_into(image, c.geometry.width, c.geometry.height, dst))
                .context("Failed to render wallpaper")?;

            c.surface.attach(Some(buffer.wl_buffer()), 0, 0);
            c.surface.damage_buffer(0, 0, c.geometry.width as i32, c.geometry.height as i32);
            c.surface.commit();

            tracing::info!(output = %c.output_name, width = c.geometry.width, height = c.geometry.height, "Wallpaper committed");

            self._committed.push((c.layer_surface, c.surface, buffer));
        }

        Ok(())
    }
}

fn output_geometry(info: &OutputInfo) -> Option<Geometry> {
    info.logical_size
        .filter(|(w, h)| *w > 0 && *h > 0)
        .map(|(w, h)| Geometry { width: w as u32, height: h as u32 })
        .or_else(|| info.modes.iter().find(|m| m.current).map(|m| Geometry { width: m.dimensions.0 as u32, height: m.dimensions.1 as u32 }))
}
