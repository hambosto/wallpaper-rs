use std::path::Path;

use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::raw::RawPool;
use smithay_client_toolkit::shm::Shm;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::Format::Xrgb8888;
use wayland_client::QueueHandle;

use crate::render::Render;

pub(super) struct PendingSurface {
    pub(super) layer_surface: LayerSurface,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) struct WaylandState {
    pub(super) registry_state: RegistryState,
    pub(super) output_state: OutputState,
    pub(super) compositor: CompositorState,
    pub(super) layer_shell: LayerShell,
    pub(super) shm: Shm,
    pub(super) pending_surfaces: Vec<PendingSurface>,
    committed: Vec<(LayerSurface, RawPool, WlBuffer)>,
}

impl WaylandState {
    pub(super) fn bind(global_list: &GlobalList, queue_handle: &QueueHandle<Self>) -> Result<Self> {
        Ok(Self {
            registry_state: RegistryState::new(global_list),
            output_state: OutputState::new(global_list, queue_handle),
            compositor: CompositorState::bind(global_list, queue_handle)
                .context("wl_compositor not available")?,
            layer_shell: LayerShell::bind(global_list, queue_handle)
                .context("zwlr_layer_shell_v1 not available")?,
            shm: Shm::bind(global_list, queue_handle).context("wl_shm not available")?,
            pending_surfaces: Vec::new(),
            committed: Vec::new(),
        })
    }

    pub(super) fn create_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        for handle in self.output_state.outputs() {
            let Some(info) = self.output_state.info(&handle) else {
                continue;
            };

            let Some((w, h)) = info
                .logical_size
                .filter(|(w, h)| *w > 0 && *h > 0)
                .or_else(|| info.modes.iter().find(|m| m.current).map(|m| m.dimensions))
            else {
                continue;
            };

            let surface = self.compositor.create_surface(queue_handle);
            let layer_surface = self.layer_shell.create_layer_surface(
                queue_handle,
                surface,
                Layer::Background,
                Some("wallpaper-rs"),
                Some(&handle),
            );

            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            layer_surface.commit();

            self.pending_surfaces.push(PendingSurface {
                layer_surface,
                width: w.cast_unsigned(),
                height: h.cast_unsigned(),
            });
        }

        tracing::info!(count = self.pending_surfaces.len(), "surfaces created");
    }

    pub(super) fn set_wallpapers(
        &mut self,
        image: &Path,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        if self.pending_surfaces.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        tracing::info!(count = self.pending_surfaces.len(), image = %image.display(), "applying wallpapers");
        let renderer = Render::new(image)?;
        let surfaces = std::mem::take(&mut self.pending_surfaces);

        for surface in surfaces {
            self.commit_wallpaper(surface, &renderer, queue_handle)?;
        }

        Ok(())
    }

    fn commit_wallpaper(
        &mut self,
        pending: PendingSurface,
        renderer: &Render,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let stride = pending.width * 4;
        let mut pool = RawPool::new((stride * pending.height) as usize, &self.shm)
            .context("failed to create shm pool")?;
        let pixels = pool.mmap();
        renderer.render(pending.width, pending.height, pixels);

        let buffer = pool.create_buffer::<WaylandState, ()>(
            0,
            pending.width.cast_signed(),
            pending.height.cast_signed(),
            stride.cast_signed(),
            Xrgb8888,
            (),
            queue_handle,
        );

        let wl_surface = pending.layer_surface.wl_surface();
        wl_surface.attach(Some(&buffer), 0, 0);
        wl_surface.damage_buffer(
            0,
            0,
            pending.width.cast_signed(),
            pending.height.cast_signed(),
        );
        pending.layer_surface.commit();

        self.committed.push((pending.layer_surface, pool, buffer));

        Ok(())
    }
}
