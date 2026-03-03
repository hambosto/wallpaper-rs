use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;

use super::state::WaylandState;

pub struct BoundGlobals {
    pub compositor: CompositorState,
    pub shm: Shm,
    pub layer_shell: LayerShell,
}

pub fn bind_globals(globals: &GlobalList, qh: &QueueHandle<WaylandState>) -> Result<BoundGlobals> {
    let compositor = CompositorState::bind(globals, qh).context("wl_compositor not available")?;
    let shm = Shm::bind(globals, qh).context("wl_shm not available")?;
    let layer_shell = LayerShell::bind(globals, qh).context("zwlr_layer_shell_v1 not available")?;

    Ok(BoundGlobals { compositor, shm, layer_shell })
}
