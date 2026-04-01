use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use wayland_client::protocol::wl_buffer::{Event, WlBuffer};
use wayland_client::protocol::wl_output::Transform;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::state::WaylandState;

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    smithay_client_toolkit::registry_handlers!(OutputState);
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}
}

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _new_factor: i32) {}

    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _new_transform: Transform) {}

    fn frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _ime: u32) {}

    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &WlOutput) {}

    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &WlOutput) {}
}

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface, configure: LayerSurfaceConfigure, serial: u32) {
        let Some(ps) = self.pending_surfaces().iter_mut().find(|ps| &ps.layer_surface == layer) else {
            return;
        };
        ps.configure_serial = Some(serial);

        let (w, h) = configure.new_size;
        if w > 0 && h > 0 {
            ps.width = w;
            ps.height = h;
        }
    }
}

impl ShmHandler for WaylandState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl Dispatch<WlBuffer, ()> for WaylandState {
    fn event(_state: &mut Self, _buffer: &WlBuffer, _event: Event, _data: &(), _conn: &Connection, _qh: &QueueHandle<Self>) {}
}

smithay_client_toolkit::delegate_compositor!(WaylandState);
smithay_client_toolkit::delegate_output!(WaylandState);
smithay_client_toolkit::delegate_registry!(WaylandState);
smithay_client_toolkit::delegate_shm!(WaylandState);
smithay_client_toolkit::delegate_layer!(WaylandState);
