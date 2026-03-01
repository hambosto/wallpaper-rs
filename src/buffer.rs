use std::os::fd::AsRawFd;
use std::os::fd::BorrowedFd;
use std::os::fd::{FromRawFd, OwnedFd};

use anyhow::{Context, Result};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::Format::Xrgb8888;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Dispatch, QueueHandle};

pub struct ShmBuffer {
    ptr: *mut libc::c_void,
    len: usize,
    pub buffer: WlBuffer,
}

unsafe impl Send for ShmBuffer {}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.ptr, self.len) };
        self.buffer.destroy();
    }
}

pub fn allocate_with<State, F>(shm: &WlShm, width: u32, height: u32, qh: &QueueHandle<State>, fill: F) -> Result<ShmBuffer>
where
    State: Dispatch<WlShmPool, ()> + Dispatch<WlBuffer, ()> + 'static,
    F: FnOnce(&mut [u8]) -> Result<()>,
{
    let stride = width as usize * 4;
    let size = stride * height as usize;
    let fd = memfd_create("wallpaper-rs-shm").context("memfd_create")?;

    if unsafe { libc::ftruncate(fd.as_raw_fd(), size as libc::off_t) } != 0 {
        anyhow::bail!("ftruncate: {}", std::io::Error::last_os_error());
    }

    let ptr = unsafe { libc::mmap(std::ptr::null_mut(), size, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED, fd.as_raw_fd(), 0) };
    if ptr == libc::MAP_FAILED {
        anyhow::bail!("mmap: {}", std::io::Error::last_os_error());
    }

    let dst = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u8, size) };
    fill(dst)?;

    let pool = shm.create_pool(fd.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(0, width as i32, height as i32, stride as i32, Xrgb8888, qh, ());
    pool.destroy();

    Ok(ShmBuffer { ptr, len: size, buffer })
}

fn memfd_create(name: &str) -> Result<OwnedFd> {
    let cname = std::ffi::CString::new(name).context("CString")?;
    let fd = unsafe { libc::syscall(libc::SYS_memfd_create, cname.as_ptr(), 1u64) as i32 };
    if fd < 0 {
        anyhow::bail!("memfd_create syscall: {}", std::io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

trait AsFd2 {
    fn as_fd(&self) -> BorrowedFd<'_>;
}

impl AsFd2 for OwnedFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.as_raw_fd()) }
    }
}
