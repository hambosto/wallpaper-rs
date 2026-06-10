use anyhow::{Context, Result};
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use smithay_client_toolkit::shm::Shm;
use wayland_client::protocol::wl_shm::Format::Xrgb8888;
use wayland_client::protocol::wl_surface::WlSurface;

pub(crate) struct ShmBuffer {
    buffer: Buffer,
    _pool: SlotPool,
}

impl ShmBuffer {
    pub(crate) fn allocate_and_fill(shm: &Shm, width: u32, height: u32, fill: impl FnOnce(&mut [u8]) -> Result<()>) -> Result<Self> {
        let stride = width * 4;
        let mut pool = SlotPool::new((stride * height) as usize, shm).context("failed to create SHM pool")?;
        let (buffer, canvas) = pool.create_buffer(width as i32, height as i32, stride as i32, Xrgb8888).context("failed to allocate SHM buffer")?;
        fill(canvas)?;

        Ok(Self { buffer, _pool: pool })
    }

    pub(crate) fn attach_to(&self, surface: &WlSurface) -> Result<()> {
        self.buffer.attach_to(surface).context("failed to attach buffer")
    }
}
