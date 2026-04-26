use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;

use crate::state::WaylandState;

pub fn bind(globals: &GlobalList, qh: &QueueHandle<WaylandState>) -> Result<WaylandState> {
    let compositor = CompositorState::bind(globals, qh).context("wl_compositor not available")?;
    let shm = Shm::bind(globals, qh).context("wl_shm not available")?;
    let layer_shell = LayerShell::bind(globals, qh).context("zwlr_layer_shell_v1 not available")?;

    Ok(WaylandState::new(RegistryState::new(globals), OutputState::new(globals, qh), compositor, layer_shell, shm))
}
