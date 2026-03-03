use wayland_client::protocol::wl_buffer::Event as WlBufferEvent;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm_pool::Event as WlShmPoolEvent;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::state::WaylandState;

impl Dispatch<WlShmPool, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlShmPool, _: WlShmPoolEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlBuffer, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlBuffer, _: WlBufferEvent, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
