use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;

use crate::state::WaylandState;

pub struct BoundGlobals {
    pub compositor: CompositorState,
    pub shm: Shm,
    pub layer_shell: LayerShell,
}

impl BoundGlobals {
    pub fn new(globals: &GlobalList, qh: &QueueHandle<WaylandState>) -> Result<Self> {
        Ok(Self {
            compositor: CompositorState::bind(globals, qh).context("wl_compositor not available")?,
            shm: Shm::bind(globals, qh).context("wl_shm not available")?,
            layer_shell: LayerShell::bind(globals, qh).context("zwlr_layer_shell_v1 not available")?,
        })
    }

    pub fn into_wayland_state(self, globals: &GlobalList, qh: &QueueHandle<WaylandState>) -> WaylandState {
        WaylandState::new(RegistryState::new(globals), OutputState::new(globals, qh), self.compositor, self.layer_shell, self.shm)
    }
}
