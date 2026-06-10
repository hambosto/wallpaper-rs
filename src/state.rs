use std::path::Path;

use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::{OutputInfo, OutputState};
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;

use crate::shm::ShmBuffer;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Geometry {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

pub(crate) struct Unconfigured {
    pub(crate) layer_surface: LayerSurface,
    pub(crate) geometry: Geometry,
    pub(crate) output_name: String,
}

impl Unconfigured {
    fn configure(self) -> Configured {
        Configured { layer_surface: self.layer_surface, geometry: self.geometry, output_name: self.output_name }
    }
}

struct Configured {
    layer_surface: LayerSurface,
    geometry: Geometry,
    output_name: String,
}

pub(crate) struct WaylandState {
    pub(crate) registry_state: RegistryState,
    pub(crate) output_state: OutputState,
    pub(crate) compositor: CompositorState,
    pub(crate) layer_shell: LayerShell,
    pub(crate) shm: Shm,
    pub(crate) unconfigured: Vec<Unconfigured>,
    committed: Vec<(LayerSurface, ShmBuffer)>,
}

impl WaylandState {
    pub(crate) fn bind(globals: &GlobalList, qh: &QueueHandle<Self>) -> Result<Self> {
        Ok(Self {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, qh),
            compositor: CompositorState::bind(globals, qh).context("wl_compositor not available")?,
            layer_shell: LayerShell::bind(globals, qh).context("zwlr_layer_shell_v1 not available")?,
            shm: Shm::bind(globals, qh).context("wl_shm not available")?,
            unconfigured: Vec::new(),
            committed: Vec::new(),
        })
    }

    pub(crate) fn create_surfaces(&mut self, qh: &QueueHandle<Self>) {
        let outputs: Vec<_> = self
            .output_state
            .outputs()
            .filter_map(|handle| {
                let info = self.output_state.info(&handle)?;
                let geometry = output_geometry(&info)?;
                let name = info.name.as_deref().map_or_else(|| format!("output-{}", info.id), String::from);
                Some((handle, geometry, name))
            })
            .collect();

        for (handle, geometry, name) in outputs {
            let surface = self.compositor.create_surface(qh);
            let layer_surface = self.layer_shell.create_layer_surface(qh, surface, Layer::Background, Some("wallpaper-rs"), Some(&handle));
            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            layer_surface.commit();
            self.unconfigured.push(Unconfigured { layer_surface, geometry, output_name: name });
        }

        tracing::info!("Surfaces created: {}", self.unconfigured.len());
    }

    pub(crate) fn set_wallpapers(&mut self, image: &Path) -> Result<()> {
        let configured: Vec<Configured> = self.unconfigured.drain(..).map(Unconfigured::configure).collect();

        if configured.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        for c in configured {
            let buffer = ShmBuffer::allocate_and_fill(&self.shm, c.geometry.width, c.geometry.height, |dst| crate::render::render_into(image, c.geometry.width, c.geometry.height, dst))
                .context("failed to render wallpaper")?;

            buffer.attach_to(c.layer_surface.wl_surface())?;
            c.layer_surface.wl_surface().damage_buffer(0, 0, c.geometry.width.cast_signed(), c.geometry.height.cast_signed());
            c.layer_surface.commit();

            tracing::info!(
                output = %c.output_name,
                width = c.geometry.width,
                height = c.geometry.height,
                "Wallpaper committed"
            );

            self.committed.push((c.layer_surface, buffer));
        }

        Ok(())
    }
}

fn output_geometry(info: &OutputInfo) -> Option<Geometry> {
    info.logical_size
        .filter(|(w, h)| *w > 0 && *h > 0)
        .map(|(w, h)| Geometry { width: w.cast_unsigned(), height: h.cast_unsigned() })
        .or_else(|| {
            info.modes
                .iter()
                .find(|m| m.current)
                .map(|m| Geometry { width: m.dimensions.0.cast_unsigned(), height: m.dimensions.1.cast_unsigned() })
        })
}
