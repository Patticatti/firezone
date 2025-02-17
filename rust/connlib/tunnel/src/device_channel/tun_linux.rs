use super::utils;
use crate::device_channel::ioctl;
use libc::{
    close, fcntl, makedev, mknod, open, F_GETFL, F_SETFL, IFF_NO_PI, IFF_TUN, O_NONBLOCK, O_RDWR,
    S_IFCHR,
};
use std::path::Path;
use std::task::{Context, Poll};
use std::{
    ffi::CStr,
    fs, io,
    os::{
        fd::{AsRawFd, RawFd},
        unix::fs::PermissionsExt,
    },
};
use tokio::io::unix::AsyncFd;

const TUNSETIFF: libc::c_ulong = 0x4004_54ca;
const TUN_DEV_MAJOR: u32 = 10;
const TUN_DEV_MINOR: u32 = 200;
const IFACE_NAME: &str = "tun-firezone"; // Keep this synced with `TunDeviceManager` until we fix the module dependencies (i.e. move `Tun` out of `firezone-tunnel`).

// Safety: We know that this is a valid C string.
const TUN_FILE: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"/dev/net/tun\0") };

#[derive(Debug)]
pub struct Tun {
    fd: AsyncFd<RawFd>,
}

impl Drop for Tun {
    fn drop(&mut self) {
        unsafe { close(self.fd.as_raw_fd()) };
    }
}

impl Tun {
    pub fn write4(&self, buf: &[u8]) -> io::Result<usize> {
        write(self.fd.as_raw_fd(), buf)
    }

    pub fn write6(&self, buf: &[u8]) -> io::Result<usize> {
        write(self.fd.as_raw_fd(), buf)
    }

    pub fn poll_read(&mut self, buf: &mut [u8], cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        utils::poll_raw_fd(&self.fd, |fd| read(fd, buf), cx)
    }

    /// Create a new [`Tun`] from a raw file descriptor.
    ///
    /// # Safety
    ///
    /// The file descriptor must be open.
    pub unsafe fn from_fd(fd: RawFd) -> io::Result<Self> {
        Ok(Tun {
            fd: AsyncFd::new(fd)?,
        })
    }

    pub fn new() -> io::Result<Self> {
        create_tun_device()?;

        let fd = match unsafe { open(TUN_FILE.as_ptr() as _, O_RDWR) } {
            -1 => return Err(get_last_error()),
            fd => fd,
        };

        // Safety: We just opened the file descriptor.
        unsafe {
            ioctl::exec(
                fd,
                TUNSETIFF,
                &mut ioctl::Request::<SetTunFlagsPayload>::new(),
            )?;
        }

        set_non_blocking(fd)?;

        // Safety: We just opened the fd.
        unsafe { Self::from_fd(fd) }
    }

    pub fn name(&self) -> &str {
        IFACE_NAME
    }
}

fn get_last_error() -> io::Error {
    io::Error::last_os_error()
}

fn set_non_blocking(fd: RawFd) -> io::Result<()> {
    match unsafe { fcntl(fd, F_GETFL) } {
        -1 => Err(get_last_error()),
        flags => match unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } {
            -1 => Err(get_last_error()),
            _ => Ok(()),
        },
    }
}

fn create_tun_device() -> io::Result<()> {
    let path = Path::new(TUN_FILE.to_str().expect("path is valid utf-8"));

    if path.exists() {
        return Ok(());
    }

    let parent_dir = path.parent().unwrap();
    fs::create_dir_all(parent_dir)?;
    let permissions = fs::Permissions::from_mode(0o751);
    fs::set_permissions(parent_dir, permissions)?;
    if unsafe {
        mknod(
            TUN_FILE.as_ptr() as _,
            S_IFCHR,
            makedev(TUN_DEV_MAJOR, TUN_DEV_MINOR),
        )
    } != 0
    {
        return Err(get_last_error());
    }

    Ok(())
}

/// Read from the given file descriptor in the buffer.
fn read(fd: RawFd, dst: &mut [u8]) -> io::Result<usize> {
    // Safety: Within this module, the file descriptor is always valid.
    match unsafe { libc::read(fd, dst.as_mut_ptr() as _, dst.len()) } {
        -1 => Err(io::Error::last_os_error()),
        n => Ok(n as usize),
    }
}

/// Write the buffer to the given file descriptor.
fn write(fd: RawFd, buf: &[u8]) -> io::Result<usize> {
    // Safety: Within this module, the file descriptor is always valid.
    match unsafe { libc::write(fd, buf.as_ptr() as _, buf.len() as _) } {
        -1 => Err(io::Error::last_os_error()),
        n => Ok(n as usize),
    }
}

impl ioctl::Request<SetTunFlagsPayload> {
    fn new() -> Self {
        let name_as_bytes = IFACE_NAME.as_bytes();
        debug_assert!(name_as_bytes.len() < libc::IF_NAMESIZE);

        let mut name = [0u8; libc::IF_NAMESIZE];
        name[..name_as_bytes.len()].copy_from_slice(name_as_bytes);

        Self {
            name,
            payload: SetTunFlagsPayload {
                flags: (IFF_TUN | IFF_NO_PI) as _,
            },
        }
    }
}

#[repr(C)]
struct SetTunFlagsPayload {
    flags: std::ffi::c_short,
}
