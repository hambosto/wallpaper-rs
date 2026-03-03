use anyhow::{Context, Result};
use wayland_client::globals::{GlobalList, GlobalListContents};
use wayland_client::protocol::wl_compositor::Event as WlCompositorEvent;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_output::Event as WlOutputEvent;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_registry::Event as WlRegistryEvent;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_shm::Event as WlShmEvent;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event as WlrLayerShellEvent;
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

    let output_interface_name = WlOutput::interface().name;
    let outputs_to_bind: Vec<_> = globals.contents().clone_list().into_iter().filter(|g| g.interface == output_interface_name).collect();

    for g in outputs_to_bind {
        let handle = globals.bind(qh, 4..=4, g.name).or_else(|_| globals.bind(qh, 1..=3, g.name)).context("wl_output bind failed")?;
        state.outputs.entry(g.name).or_default().handle = Some(handle);
    }

    state.globals = Some(BoundGlobals { compositor, shm, layer_shell });
    Ok(())
}

impl Dispatch<WlRegistry, GlobalListContents> for WaylandState {
    fn event(_: &mut Self, _: &WlRegistry, _: WlRegistryEvent, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlCompositor, _: WlCompositorEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlShm, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlShm, _: WlShmEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlOutput, u32> for WaylandState {
    fn event(state: &mut Self, _: &WlOutput, event: WlOutputEvent, id: &u32, _: &Connection, _: &QueueHandle<Self>) {
        let info = state.outputs.entry(*id).or_default();
        match event {
            WlOutputEvent::Name { name } => info.name = Some(name),
            WlOutputEvent::Mode { width, height, .. } => {
                info.width = width as u32;
                info.height = height as u32;
            }
            WlOutputEvent::Done => info.configured = true,
            _ => {}
        }
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(_: &mut Self, _: &ZwlrLayerShellV1, _: WlrLayerShellEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
