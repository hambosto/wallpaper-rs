use anyhow::{Context, Result};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::protocol::wl_shm::Format;

use crate::transition::Transition;

pub(super) struct Surface {
    layer_surface: LayerSurface,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) pixels: Vec<u8>,
    pub(super) pool: Option<SlotPool>,
    pub(super) transition: Option<Transition>,
}

impl Surface {
    pub(super) fn new(layer_surface: LayerSurface, width: u32, height: u32) -> Self {
        Self { layer_surface, width, height, pixels: Vec::new(), pool: None, transition: None }
    }

    pub(super) fn tick(&mut self) -> bool {
        let Some(transition) = &mut self.transition else {
            return false;
        };

        let finished = transition.frame(&mut self.pixels);
        if finished {
            let (w, h) = transition.dimensions();
            tracing::info!(width = w, height = h, "transition completed");
            self.transition = None;
            self.pool = None;
        }

        !finished
    }

    pub(super) fn commit(&mut self, shm: &Shm) -> Result<()> {
        let needed = self.width.saturating_mul(4).saturating_mul(self.height) as usize;

        let current_len = self.pool.as_ref().map_or(0, SlotPool::len);
        if current_len < needed {
            self.pool = None;
        }

        if self.pool.is_none() {
            let pool = SlotPool::new(needed, shm).context("failed to allocate shm pool for commit")?;
            self.pool = Some(pool);
        }

        let pool = self.pool.as_mut().context("shm pool not initialized")?;
        let width = self.width.cast_signed();
        let height = self.height.cast_signed();
        let stride = self.width.saturating_mul(4).cast_signed();

        let (buffer, canvas) = pool.create_buffer(width, height, stride, Format::Xrgb8888).context("failed to create buffer")?;
        if canvas.len() != self.pixels.len() {
            anyhow::bail!("canvas size {} does not match pixel buffer size {}", canvas.len(), self.pixels.len());
        }
        canvas.copy_from_slice(&self.pixels);

        let wl_surface = self.layer_surface.wl_surface();
        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, width, height);

        self.layer_surface.commit();

        Ok(())
    }
}
