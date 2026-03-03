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
    pub fn new<F>(shm: &Shm, width: u32, height: u32, qh: &QueueHandle<WaylandState>, fill: F) -> Result<Self>
    where
        F: FnOnce(&mut [u8]) -> Result<()>,
    {
        let layout = BufferLayout::new(width, height);

        let mut pool = RawPool::new(layout.size, shm).context("Failed to create SHM pool")?;
        fill(pool_bytes(&mut pool, layout.size))?;
        let buffer = pool.create_buffer(0, layout.width, layout.height, layout.stride, Xrgb8888, (), qh);

        Ok(Self { _pool: pool, buffer })
    }

    pub fn buffer(&self) -> &WlBuffer {
        &self.buffer
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

struct BufferLayout {
    width: i32,
    height: i32,
    stride: i32,
    size: usize,
}

impl BufferLayout {
    fn new(width: u32, height: u32) -> Self {
        let stride = width as i32 * 4;
        let size = stride as usize * height as usize;
        Self { width: width as i32, height: height as i32, stride, size }
    }
}

fn pool_bytes(pool: &mut RawPool, size: usize) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(pool.mmap().as_ptr() as *mut u8, size) }
}
