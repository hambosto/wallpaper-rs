use smithay_client_toolkit::output::OutputState;
use wayland_client::protocol::wl_output::WlOutput;

pub struct ResolvedOutput {
    pub name: String,
    pub handle: WlOutput,
    pub width: u32,
    pub height: u32,
}

pub fn resolve(output_state: &OutputState) -> Vec<ResolvedOutput> {
    output_state
        .outputs()
        .filter_map(|output| {
            let info = output_state.info(&output)?;
            let name = info.name.clone().unwrap_or_else(|| format!("output-{}", info.id));

            let (width, height) = info
                .logical_size
                .filter(|(w, h)| *w > 0 && *h > 0)
                .map(|(w, h)| (w as u32, h as u32))
                .or_else(|| info.modes.iter().find(|m| m.current).map(|m| (m.dimensions.0 as u32, m.dimensions.1 as u32)))
                .or_else(|| {
                    println!("warning: output '{name}' (id={}) skipped: no valid mode", info.id);
                    None
                })?;

            Some(ResolvedOutput { name, handle: output, width, height })
        })
        .collect()
}
