use anyhow::{Context, Result};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::Format::Xrgb8888;

pub struct ShmBuffer {
    buffer: Buffer,
    _pool: SlotPool,
}

impl ShmBuffer {
    pub fn new<F>(shm: &Shm, width: u32, height: u32, fill: F) -> Result<Self>
    where
        F: FnOnce(&mut [u8]),
    {
        let stride = width * 4;
        let size = (stride * height) as usize;

        let mut pool = SlotPool::new(size, shm).context("Failed to create SHM pool")?;

        let (buffer, canvas) = pool.create_buffer(width as i32, height as i32, stride as i32, Xrgb8888).context("Failed to allocate buffer slot")?;

        fill(unsafe { std::slice::from_raw_parts_mut(canvas.as_mut_ptr().cast::<u8>(), size) });

        Ok(Self { buffer, _pool: pool })
    }

    #[inline]
    pub fn wl_buffer(&self) -> &WlBuffer {
        self.buffer.wl_buffer()
    }
}
