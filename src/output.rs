use smithay_client_toolkit::output::{OutputInfo, OutputState};
use wayland_client::protocol::wl_output::WlOutput;

pub struct ResolvedOutput {
    pub name: String,
    pub handle: WlOutput,
    pub width: u32,
    pub height: u32,
}

impl ResolvedOutput {
    pub fn resolve_all(output_state: &OutputState) -> Vec<Self> {
        output_state.outputs().filter_map(|handle| Self::from_handle(output_state, handle)).collect()
    }

    fn from_handle(output_state: &OutputState, handle: WlOutput) -> Option<Self> {
        let info = output_state.info(&handle)?;
        let name = info.name.as_deref().map(String::from).unwrap_or_else(|| format!("output-{}", info.id));
        let (width, height) = dimensions(&info)?;

        Some(Self { name, handle, width, height })
    }
}

fn dimensions(info: &OutputInfo) -> Option<(u32, u32)> {
    info.logical_size
        .filter(|(w, h)| *w > 0 && *h > 0)
        .map(|(w, h)| (w as u32, h as u32))
        .or_else(|| info.modes.iter().find(|m| m.current).map(|m| (m.dimensions.0 as u32, m.dimensions.1 as u32)))
}
