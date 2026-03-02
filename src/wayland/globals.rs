use anyhow::{Context, Result};
use wayland_client::globals::{GlobalList, GlobalListContents};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use super::state::WaylandState;

pub struct BoundGlobals {
    pub compositor: WlCompositor,
    pub shm: WlShm,
    pub layer_shell: ZwlrLayerShellV1,
}

pub fn bind_all(globals: &GlobalList, qh: &QueueHandle<WaylandState>, state: &mut WaylandState) -> Result<()> {
    let compositor: WlCompositor = globals.bind(qh, 4..=6, ()).or_else(|_| globals.bind(qh, 1..=3, ())).context("wl_compositor not available")?;

    let shm: WlShm = globals.bind(qh, 1..=1, ()).context("wl_shm not available")?;

    let layer_shell: ZwlrLayerShellV1 = globals.bind(qh, 1..=4, ()).context("zwlr_layer_shell_v1 not available — is your compositor sway/hyprland/river?")?;

    for g in globals.contents().with_list(|l| l.to_vec()) {
        if g.interface != "wl_output" {
            continue;
        }
        let handle = globals.bind(qh, 4..=4, g.name).or_else(|_| globals.bind(qh, 1..=3, g.name)).context("wl_output bind failed")?;

        state.outputs.entry(g.name).or_default().handle = Some(handle);
    }

    state.globals = Some(BoundGlobals { compositor, shm, layer_shell });
    Ok(())
}

impl Dispatch<WlRegistry, GlobalListContents> for WaylandState {
    fn event(_: &mut Self, _: &WlRegistry, _: wayland_client::protocol::wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlCompositor, _: wayland_client::protocol::wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlShm, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlShm, _: wayland_client::protocol::wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(_: &mut Self, _: &ZwlrLayerShellV1, _: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
