use anyhow::{Context, Result};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::raw::RawPool;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::Format::Xrgb8888;

use crate::state::WaylandState;

pub struct ShmBuffer {
    _pool: RawPool,
    buffer: WlBuffer,
}

impl ShmBuffer {
    pub fn buffer(&self) -> &WlBuffer {
        &self.buffer
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

pub struct ShmBufferBuilder<'a> {
    shm: &'a Shm,
    width: u32,
    height: u32,
    qh: &'a QueueHandle<WaylandState>,
}

impl<'a> ShmBufferBuilder<'a> {
    pub fn new(shm: &'a Shm, width: u32, height: u32, qh: &'a QueueHandle<WaylandState>) -> Self {
        Self { shm, width, height, qh }
    }

    pub fn build_with<F>(self, fill: F) -> Result<ShmBuffer>
    where
        F: FnOnce(&mut [u8]) -> Result<()>,
    {
        let stride = self.width as i32 * 4;
        let size = stride as usize * self.height as usize;

        let mut pool = RawPool::new(size, self.shm).context("Failed to create SHM pool")?;

        {
            let data = pool.mmap();
            let bytes = unsafe { std::slice::from_raw_parts_mut(data.as_ptr() as *mut u8, size) };
            fill(bytes)?;
        }

        let buffer = pool.create_buffer(0, self.width as i32, self.height as i32, stride, Xrgb8888, (), self.qh);

        Ok(ShmBuffer { _pool: pool, buffer })
    }
}
