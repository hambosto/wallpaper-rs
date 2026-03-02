use std::collections::HashMap;

use wayland_client::protocol::wl_output::{Event, WlOutput};
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::state::WaylandState;

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
        (self.configured && self.width > 0 && self.height > 0).then_some((self.width, self.height))
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

pub fn resolve(outputs: &HashMap<u32, OutputInfo>) -> Vec<ResolvedOutput> {
    outputs
        .iter()
        .filter_map(|(id, info)| {
            let handle = info.handle.clone()?;
            let (width, height) = info.size().or_else(|| {
                let name = info.name.as_deref().unwrap_or("?");
                if !info.configured {
                    println!("warning: output '{name}' (id={id}) skipped: wl_output::Done not received");
                } else {
                    println!("warning: output '{name}' (id={id}) skipped: no valid mode ({}x{})", info.width, info.height);
                }
                None
            })?;

            Some(ResolvedOutput { name: info.name.clone().unwrap_or_else(|| format!("output-{id}")), handle, width, height })
        })
        .collect()
}
