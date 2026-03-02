use std::collections::HashMap;

use super::globals::BoundGlobals;
use super::output::OutputInfo;
use super::surface::PendingSurface;
use crate::shm::ShmBuffer;

#[derive(Default)]
pub struct WaylandState {
    pub globals: Option<BoundGlobals>,
    pub outputs: HashMap<u32, OutputInfo>,
    pub pending: Vec<PendingSurface>,
    pub buffers: Vec<ShmBuffer>,
}
