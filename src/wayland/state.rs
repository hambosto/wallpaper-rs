use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;

use super::surface::PendingSurface;
use crate::shm::ShmBuffer;

pub struct WaylandState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub pending: Vec<PendingSurface>,
    pub buffers: Vec<ShmBuffer>,
}
