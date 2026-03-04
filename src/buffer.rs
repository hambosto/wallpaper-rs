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
        let layout = BufferLayout::new(width, height);

        let mut pool = SlotPool::new(layout.size, shm).context("Failed to create SHM pool")?;
        let (buffer, canvas) = pool.create_buffer(layout.width, layout.height, layout.stride, Xrgb8888).context("Failed to create buffer")?;

        let canvas = unsafe {
            let ptr = canvas.as_ptr() as *mut u8;
            std::slice::from_raw_parts_mut(ptr, layout.size)
        };
        fill(canvas);

        Ok(Self { buffer, _pool: pool })
    }

    pub fn buffer(&self) -> &WlBuffer {
        self.buffer.wl_buffer()
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
