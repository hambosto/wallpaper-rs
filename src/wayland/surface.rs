use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_surface::Event as WlSurfaceEvent;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{Anchor, Event, ZwlrLayerSurfaceV1};

use super::output::ResolvedOutput;
use super::state::WaylandState;

pub struct PendingSurface {
    pub layer_surface: ZwlrLayerSurfaceV1,
    pub surface: WlSurface,
    pub configure_serial: Option<u32>,
    pub output_name: String,
    pub width: u32,
    pub height: u32,
}

pub fn create_for_outputs(compositor: &WlCompositor, layer_shell: &ZwlrLayerShellV1, outputs: &[ResolvedOutput], state: &mut WaylandState, qh: &QueueHandle<WaylandState>) {
    for output in outputs {
        let surface = compositor.create_surface(qh, ());
        let idx = state.pending.len();
        let layer_surface = layer_shell.get_layer_surface(&surface, Some(&output.handle), Layer::Background, "wallpaper-rs".into(), qh, idx);

        layer_surface.set_anchor(Anchor::all());
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_size(0, 0);
        surface.commit();

        state
            .pending
            .push(PendingSurface { layer_surface, surface, configure_serial: None, output_name: output.name.clone(), width: output.width, height: output.height });
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, usize> for WaylandState {
    fn event(state: &mut Self, _: &ZwlrLayerSurfaceV1, event: Event, idx: &usize, _: &Connection, _: &QueueHandle<Self>) {
        if let Event::Configure { serial, width, height } = event
            && let Some(ps) = state.pending.get_mut(*idx)
        {
            ps.configure_serial = Some(serial);
            if width > 0 && height > 0 {
                ps.width = width;
                ps.height = height;
            }
        }
    }
}

impl Dispatch<WlSurface, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlSurface, _: WlSurfaceEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
