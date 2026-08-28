use std::{fmt::Debug, io};

use bitflags::bitflags;
use drm_ffi::{DRM_COMMAND_BASE, DRM_IOCTL_BASE};
use rustix::ioctl::{self, Opcode};

use crate::device::TegraDRM;

bitflags! {
    #[allow(nonstandard_style)]
     pub struct drm_tegra_gem_create_flags: u32 {
        const TILED = 1 << 0;
        const BOTTOM_UP = 1 << 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_gem_create {
    size: u64,
    flags: drm_tegra_gem_create_flags,
    handle: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_gem_mmap {
    handle: u32,
    pad: u32,
    offset: u64,
}

const DRM_IOCTL_TEGRA_GEM_CREATE: Opcode =
    ioctl::opcode::read_write::<drm_tegra_gem_create>(DRM_IOCTL_BASE, DRM_COMMAND_BASE as u8);
const DRM_IOCTL_TEGRA_GEM_MMAP: Opcode =
    ioctl::opcode::read_write::<drm_tegra_gem_mmap>(DRM_IOCTL_BASE, DRM_COMMAND_BASE as u8 + 0x01);

pub fn gem_create(fd: &TegraDRM, size: u64, flags: drm_tegra_gem_create_flags) -> io::Result<u32> {
    let mut v = drm_tegra_gem_create {
        size,
        flags,
        handle: 0,
    };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_GEM_CREATE, _>::new(&mut v),
        )?;
    }
    Ok(v.handle)
}

pub fn gem_mmap(fd: &TegraDRM, handle: u32) -> io::Result<u64> {
    let mut v = drm_tegra_gem_mmap {
        handle,
        offset: 0,
        pad: 0,
    };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_GEM_MMAP, _>::new(&mut v),
        )?
    };
    Ok(v.offset)
}
