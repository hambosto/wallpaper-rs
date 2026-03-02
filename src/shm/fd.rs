use anyhow::{Context, Result};
use rustix::fd::OwnedFd;
use rustix::fs::{MemfdFlags, SealFlags};

pub fn create_shm_fd(size: usize) -> Result<OwnedFd> {
    let fd = rustix::fs::memfd_create(c"wallpaper-rs", MemfdFlags::ALLOW_SEALING | MemfdFlags::CLOEXEC).context("memfd_create")?;

    rustix::fs::ftruncate(&fd, size as u64).context("ftruncate")?;

    let _ = rustix::fs::fcntl_add_seals(&fd, SealFlags::SHRINK | SealFlags::SEAL);

    Ok(fd)
}
