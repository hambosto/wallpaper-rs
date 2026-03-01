use wayland_client::protocol::wl_output::{Event, WlOutput};
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::wayland::WaylandState;

#[derive(Default)]
pub struct OutputInfo {
    pub handle: Option<WlOutput>,
    pub name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub configured: bool,
}

impl OutputInfo {
    pub fn size(&self) -> Option<(u32, u32)> {
        if !self.configured || self.width == 0 || self.height == 0 {
            return None;
        }
        Some((self.width, self.height))
    }
}

pub struct ResolvedOutput {
    pub name: String,
    pub handle: WlOutput,
    pub width: u32,
    pub height: u32,
}

impl Dispatch<WlOutput, u32> for WaylandState {
    fn event(state: &mut Self, _: &WlOutput, event: Event, id: &u32, _: &Connection, _: &QueueHandle<Self>) {
        let info = state.outputs.entry(*id).or_default();
        match event {
            Event::Name { name } => info.name = Some(name),
            Event::Mode { width, height, .. } => {
                info.width = width as u32;
                info.height = height as u32;
            }
            Event::Done => info.configured = true,
            _ => {}
        }
    }
}
