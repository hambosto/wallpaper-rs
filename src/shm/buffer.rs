use core::ptr::NonNull;

use anyhow::{Context, Result};
use rustix::fd::AsFd;
use rustix::mm::{MapFlags, ProtFlags};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::Format::Xrgb8888;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Dispatch, QueueHandle};

use super::fd::create_shm_fd;

pub struct ShmBuffer {
    ptr: NonNull<core::ffi::c_void>,
    len: usize,
    pub buffer: WlBuffer,
}

unsafe impl Send for ShmBuffer {}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        let _ = unsafe { rustix::mm::munmap(self.ptr.as_ptr(), self.len) };
        self.buffer.destroy();
    }
}

pub struct ShmBufferBuilder<'a, State> {
    shm: &'a WlShm,
    width: u32,
    height: u32,
    qh: &'a QueueHandle<State>,
}

impl<'a, State> ShmBufferBuilder<'a, State>
where
    State: Dispatch<WlShmPool, ()> + Dispatch<WlBuffer, ()> + 'static,
{
    pub fn new(shm: &'a WlShm, width: u32, height: u32, qh: &'a QueueHandle<State>) -> Self {
        Self { shm, width, height, qh }
    }

    pub fn build_with<F>(self, fill: F) -> Result<ShmBuffer>
    where
        F: FnOnce(&mut [u8]) -> Result<()>,
    {
        let stride = self.width as usize * 4;
        let size = stride * self.height as usize;

        let fd = create_shm_fd(size)?;

        let raw_ptr = unsafe { rustix::mm::mmap(core::ptr::null_mut(), size, ProtFlags::READ | ProtFlags::WRITE, MapFlags::SHARED, &fd, 0) }.context("mmap")?;

        let ptr = NonNull::new(raw_ptr).context("mmap returned null")?;

        {
            let bytes = unsafe { core::slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, size) };
            fill(bytes)?;
        }

        let pool = self.shm.create_pool(fd.as_fd(), size as i32, self.qh, ());
        let buffer = pool.create_buffer(0, self.width as i32, self.height as i32, stride as i32, Xrgb8888, self.qh, ());
        pool.destroy();

        Ok(ShmBuffer { ptr, len: size, buffer })
    }
}
