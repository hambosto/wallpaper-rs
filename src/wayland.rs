use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use wayland_client::globals::GlobalListContents;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use crate::buffer::{self, ShmBuffer};
use crate::config::Config;
use crate::output::{OutputInfo, ResolvedOutput};
use crate::renderer;
use crate::surface::{self, PendingSurface};

struct Globals {
    compositor: WlCompositor,
    shm: WlShm,
    layer_shell: ZwlrLayerShellV1,
}

pub struct WaylandState {
    globals: Option<Globals>,
    pub outputs: HashMap<u32, OutputInfo>,
    pub pending: Vec<PendingSurface>,
    buffers: Vec<ShmBuffer>,
}

impl WaylandState {
    fn new() -> Self {
        Self { globals: None, outputs: HashMap::new(), pending: Vec::new(), buffers: Vec::new() }
    }
}

pub fn run(config: &Config) -> Result<()> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland display")?;

    let (global_list, mut event_queue) = wayland_client::globals::registry_queue_init::<WaylandState>(&conn).map_err(|e| anyhow::anyhow!("{e:?}"))?;

    let queue_handle = event_queue.handle();
    let mut state = WaylandState::new();

    bind_globals(&global_list, &queue_handle, &mut state)?;

    event_queue.roundtrip(&mut state).context("Initial roundtrip")?;
    event_queue.roundtrip(&mut state).context("Output-done roundtrip")?;

    let (compositor, shm, layer_shell) = take_globals(&state)?;

    let outputs = resolve_outputs(&state.outputs);
    if outputs.is_empty() {
        let total = state.outputs.len();
        if total > 0 {
            bail!("{total} wl_output(s) found but none finished configuring (no wl_output::Done received) — try adding an extra roundtrip");
        }
        bail!("No wl_output objects found");
    }

    surface::create_for_outputs(&compositor, &layer_shell, &outputs, &mut state, &queue_handle);
    event_queue.roundtrip(&mut state).context("Configure roundtrip")?;

    let set_wallpaper = commit_wallpapers(&shm, &mut state, config, &queue_handle)?;
    if set_wallpaper == 0 {
        bail!("No wallpapers were set — check your config");
    }

    event_queue.roundtrip(&mut state).context("Final roundtrip")?;
    println!("Done — {set_wallpaper} output(s) wallpapered");

    loop {
        event_queue.blocking_dispatch(&mut state).context("Wayland dispatch error")?;
    }
}

fn bind_globals(globals: &wayland_client::globals::GlobalList, qh: &QueueHandle<WaylandState>, state: &mut WaylandState) -> Result<()> {
    let compositor: WlCompositor = globals.bind(qh, 4..=6, ()).or_else(|_| globals.bind(qh, 1..=3, ())).context("wl_compositor not available")?;
    let shm: WlShm = globals.bind(qh, 1..=1, ()).context("wl_shm not available")?;
    let layer_shell: ZwlrLayerShellV1 = globals.bind(qh, 1..=4, ()).context("zwlr_layer_shell_v1 not available — is your compositor sway/hyprland/river?")?;

    for g in globals.contents().with_list(|l| l.to_vec()) {
        if g.interface != "wl_output" {
            continue;
        }
        let handle = globals.bind(qh, 4..=4, g.name).or_else(|_| globals.bind(qh, 1..=3, g.name)).context("wl_output bind failed")?;
        state.outputs.entry(g.name).or_default().handle = Some(handle);
    }

    state.globals = Some(Globals { compositor, shm, layer_shell });

    Ok(())
}

fn take_globals(state: &WaylandState) -> Result<(WlCompositor, WlShm, ZwlrLayerShellV1)> {
    let g = state.globals.as_ref().context("Required Wayland globals not bound")?;
    Ok((g.compositor.clone(), g.shm.clone(), g.layer_shell.clone()))
}

fn resolve_outputs(outputs: &HashMap<u32, OutputInfo>) -> Vec<ResolvedOutput> {
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

fn commit_wallpapers(shm: &WlShm, state: &mut WaylandState, config: &Config, qh: &QueueHandle<WaylandState>) -> Result<usize> {
    let pending = std::mem::take(&mut state.pending);
    let mut count = 0;

    for pending_surface in pending {
        let Some(serial) = pending_surface.configure_serial else {
            println!("warning: no configure received for '{}', skipping", pending_surface.output_name);
            continue;
        };

        let path = &config.image;
        pending_surface.layer_surface.ack_configure(serial);
        let buffer = buffer::allocate_with(shm, pending_surface.width, pending_surface.height, qh, |dst| renderer::render_into(dst, path, pending_surface.width, pending_surface.height))
            .with_context(|| format!("Render+upload wallpaper for '{}'", pending_surface.output_name))?;

        pending_surface.surface.attach(Some(&buffer.buffer), 0, 0);
        pending_surface.surface.damage_buffer(0, 0, pending_surface.width as i32, pending_surface.height as i32);
        pending_surface.surface.commit();

        state.buffers.push(buffer);

        println!("wallpaper set for '{}'", pending_surface.output_name);
        count += 1;
    }

    Ok(count)
}

impl Dispatch<WlRegistry, GlobalListContents> for WaylandState {
    fn event(_: &mut Self, _: &WlRegistry, _: wayland_client::protocol::wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlCompositor, _: wayland_client::protocol::wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlShm, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlShm, _: wayland_client::protocol::wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrLayerShellV1, ()> for WaylandState {
    fn event(_: &mut Self, _: &ZwlrLayerShellV1, _: wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlShmPool, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlShmPool, _: wayland_client::protocol::wl_shm_pool::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<WlBuffer, ()> for WaylandState {
    fn event(_: &mut Self, _: &WlBuffer, _: wayland_client::protocol::wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
