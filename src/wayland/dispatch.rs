use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::ProvidesRegistryState;
use smithay_client_toolkit::registry::RegistryState;
use wayland_client::protocol::wl_buffer::Event as WlBufferEvent;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm_pool::Event as WlShmPoolEvent;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::state::WaylandState;

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

impl Dispatch<WlShmPool, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlShmPool, _: WlShmPoolEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlBuffer, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlBuffer, _: WlBufferEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

smithay_client_toolkit::delegate_output!(WaylandState);
smithay_client_toolkit::delegate_registry!(WaylandState);
