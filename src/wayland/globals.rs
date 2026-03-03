use anyhow::{Context, Result};
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_compositor::Event as WlCompositorEvent;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_shm::Event as WlShmEvent;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event as WlrLayerShellEvent;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use super::state::WaylandState;

#[derive(Clone)]
pub struct BoundGlobals {
    pub compositor: WlCompositor,
    pub shm: WlShm,
    pub layer_shell: ZwlrLayerShellV1,
}

pub fn bind_globals(globals: &GlobalList, qh: &QueueHandle<WaylandState>) -> Result<BoundGlobals> {
    let compositor: WlCompositor = globals.bind(qh, 4..=6, ()).or_else(|_| globals.bind(qh, 1..=3, ())).context("wl_compositor not available")?;
    let shm: WlShm = globals.bind(qh, 1..=1, ()).context("wl_shm not available")?;
    let layer_shell: ZwlrLayerShellV1 = globals.bind(qh, 1..=4, ()).context("zwlr_layer_shell_v1 not available — is your compositor sway/hyprland/river?")?;

    Ok(BoundGlobals { compositor, shm, layer_shell })
}

impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlCompositor, _: WlCompositorEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlShm, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlShm, _: WlShmEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(_: &mut Self, _: &ZwlrLayerShellV1, _: WlrLayerShellEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
