use std::io;

use bitflags::bitflags;
use drm::control::syncobj;
use drm_ffi::{DRM_COMMAND_BASE, DRM_IOCTL_BASE};
use host1x_core::cmdseq::ValidRawCommandArray;
use rustix::ioctl::{self, Opcode};

use crate::device::TegraDRM;

bitflags! {
    #[allow(nonstandard_style)]
    pub struct drm_tegra_channel_capabilities: u32 {
        const CACHE_COHERENT = 1 << 0;
    }

    #[allow(nonstandard_style)]
    pub struct drm_tegra_channel_map_flags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_channel_open {
    /// Host1x class of the engine that will be programmed using this channel.
    host1x_class: u32,

    /// Flags
    flags: u32,

    /// Opaque identifier corresponding to the opened channel.
    context: u32,

    /// Version of the engine hardware. This can be used by userspace
    /// to determine how the engine needs to be programmed.
    version: u32,

    /// Flags describing the hardware capabilities.
    capabilities: u32,
    padding: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_channel_close {
    /// Identifier of the channel to close.
    context: u32,
    padding: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_channel_map {
    /// Identifier of the channel to which make memory available for.
    context: u32,

    /// GEM handle of the memory to map.
    handle: u32,

    /// Flags
    flags: u32,

    /// Identifier corresponding to the mapping, to be used for relocations or unmapping later.
    mapping: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_channel_unmap {
    /// Channel identifier of the channel to unmap memory from.
    context: u32,

    /// Mapping identifier of the memory mapping to unmap.
    mapping: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_submit_syncpt {
    /// ID of the syncpoint that the job will increment.
    id: u32,

    /// Flags.
    flags: u32,

    /// Number of times the job will increment this syncpoint.
    increments: u32,

    /// Value the syncpoint will have once the job has completed all
    /// its specified syncpoint increments.
    ///
    /// Note that the kernel may increment the syncpoint before or after
    /// the job. These increments are not reflected in this field.
    ///
    /// If the job hangs or times out, not all of the increments may
    /// get executed.
    ///
    value: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_channel_submit {
    /// Identifier of the channel to submit this job to.
    context: u32,

    /// Number of elements in the `bufs_ptr` array.
    num_bufs: u32,

    /// Number of elements in the `cmds_ptr` array.
    num_cmds: u32,

    /// Number of 32-bit words in the `gather_data_ptr` array.
    gather_data_words: u32,

    /// Pointer to an array of drm_tegra_submit_buf structures.
    bufs_ptr: u64,

    /// Pointer to an array of drm_tegra_submit_cmd structures.
    cmds_ptr: u64,

    /// Pointer to an array of Host1x opcodes to be used by GATHER_UPTR commands.
    gather_data_ptr: u64,

    /// Handle for DRM syncobj that will be waited before submission.
    /// Ignored if zero.
    syncobj_in: u32,

    /// Handle for DRM syncobj that will have its fence replaced with
    /// the job's completion fence. Ignored if zero.
    syncobj_out: u32,

    /// Information about the syncpoint the job will increment.
    syncpt: drm_tegra_submit_syncpt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct drm_tegra_submit_buf {
    /// Identifier of the mapping to use in the submission.
    pub mapping: u32,

    /// Flags.
    pub flags: u32,

    /// Offset from the start of the mapping of the data whose address is to be patched into the gather.
    pub reloc_target_offset: u64,

    /// Offset in words from the start of the gather data to where the address should be patched into.
    pub reloc_gather_offset_words: u32,

    /// Number of bits the address should be shifted right before patching in.
    pub reloc_shift: u32,
}

const DRM_TEGRA_SUBMIT_CMD_GATHER_UPTR: u32 = 0;
const DRM_TEGRA_SUBMIT_CMD_WAIT_SYNCPT: u32 = 1;
const DRM_TEGRA_SUBMIT_CMD_WAIT_SYNCPT_RELATIVE: u32 = 2;

#[repr(C)]
struct drm_tegra_submit_cmd {
    kind: u32,
    flags: u32,
    un: drm_tegra_submit_cmd_union,
}

#[repr(C)]
union drm_tegra_submit_cmd_union {
    gather_uptr: drm_tegra_submit_cmd_gather_uptr,
    wait_syncpt: drm_tegra_submit_cmd_wait_syncpt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_submit_cmd_gather_uptr {
    words: u32,
    reserved: [u32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_submit_cmd_wait_syncpt {
    id: u32,
    value: u32,
    reserved: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_syncpoint_allocate {
    /// ID of allocated syncpoint.
    id: u32,
    padding: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_syncpoint_free {
    /// ID of syncpoint to free.
    id: u32,
    padding: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct drm_tegra_syncpoint_wait {
    /// Absolute timestamp at which the wait will time out.
    timeout_ns: i64,

    /// ID of syncpoint to wait on.
    id: u32,

    /// Threshold to wait for.
    threshold: u32,

    /// Value of the syncpoint upon wait completion.
    value: u32,
    padding: u32,
}

const DRM_IOCTL_TEGRA_CHANNEL_OPEN: Opcode = ioctl::opcode::read_write::<drm_tegra_channel_open>(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE as u8 + 0x10,
);
const DRM_IOCTL_TEGRA_CHANNEL_CLOSE: Opcode = ioctl::opcode::read_write::<drm_tegra_channel_close>(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE as u8 + 0x11,
);
const DRM_IOCTL_TEGRA_CHANNEL_MAP: Opcode = ioctl::opcode::read_write::<drm_tegra_channel_map>(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE as u8 + 0x12,
);
const DRM_IOCTL_TEGRA_CHANNEL_UNMAP: Opcode = ioctl::opcode::read_write::<drm_tegra_channel_unmap>(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE as u8 + 0x13,
);
const DRM_IOCTL_TEGRA_CHANNEL_SUBMIT: Opcode = ioctl::opcode::read_write::<drm_tegra_channel_submit>(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE as u8 + 0x14,
);
const DRM_IOCTL_TEGRA_SYNCPOINT_ALLOCATE: Opcode = ioctl::opcode::read_write::<
    drm_tegra_syncpoint_allocate,
>(DRM_IOCTL_BASE, DRM_COMMAND_BASE as u8 + 0x20);
const DRM_IOCTL_TEGRA_SYNCPOINT_FREE: Opcode = ioctl::opcode::read_write::<drm_tegra_syncpoint_free>(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE as u8 + 0x21,
);
const DRM_IOCTL_TEGRA_SYNCPOINT_WAIT: Opcode = ioctl::opcode::read_write::<drm_tegra_syncpoint_wait>(
    DRM_IOCTL_BASE,
    DRM_COMMAND_BASE as u8 + 0x22,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelInfo {
    pub context: u32,
    pub version: u32,
    pub capabilities: drm_tegra_channel_capabilities,
}

pub fn channel_open(fd: &TegraDRM, host1x_class: u32) -> io::Result<ChannelInfo> {
    let mut v = drm_tegra_channel_open {
        host1x_class,
        flags: 0,
        context: 0,
        version: 0,
        capabilities: 0,
        padding: 0,
    };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_CHANNEL_OPEN, _>::new(&mut v),
        )?
    }

    let drm_tegra_channel_open {
        context,
        version,
        capabilities,
        ..
    } = v;
    let capabilities = drm_tegra_channel_capabilities::from_bits_truncate(capabilities);

    Ok(ChannelInfo {
        context,
        version,
        capabilities,
    })
}

pub fn channel_map(
    fd: &TegraDRM,
    context: u32,
    handle: u32,
    flags: drm_tegra_channel_map_flags,
) -> io::Result<u32> {
    let mut v = drm_tegra_channel_map {
        context,
        handle,
        flags: flags.bits,
        mapping: 0,
    };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_CHANNEL_MAP, _>::new(&mut v),
        )?
    };
    Ok(v.mapping)
}

pub fn channel_unmap(fd: &TegraDRM, context: u32, mapping: u32) -> io::Result<()> {
    let mut v = drm_tegra_channel_unmap { context, mapping };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_CHANNEL_UNMAP, _>::new(&mut v),
        )
        .map_err(Into::into)
    }
}

pub fn channel_close(fd: &TegraDRM, context: u32) -> io::Result<()> {
    let mut v = drm_tegra_channel_close {
        context,
        padding: 0,
    };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_CHANNEL_CLOSE, _>::new(&mut v),
        )
        .map_err(Into::into)
    }
}

pub unsafe fn channel_submit(
    fd: &TegraDRM,
    context: u32,
    sp_id: u32,
    sp_increments: u32,
    buf: ValidRawCommandArray<'_>,
    relocs: &[drm_tegra_submit_buf],
    syncobj_in: Option<syncobj::Handle>,
    syncobj_out: Option<syncobj::Handle>,
) -> io::Result<u32> {
    let words = buf.len().try_into().unwrap();
    debug_assert!(words != 0, "attempt to submit empty command buffer");
    let cmds = [drm_tegra_submit_cmd {
        kind: DRM_TEGRA_SUBMIT_CMD_GATHER_UPTR,
        flags: 0,
        un: drm_tegra_submit_cmd_union {
            gather_uptr: drm_tegra_submit_cmd_gather_uptr {
                words,
                reserved: [0; _],
            },
        },
    }];

    let mut v = drm_tegra_channel_submit {
        context,
        num_bufs: relocs.len() as _,
        num_cmds: cmds.len() as _,
        gather_data_words: words,
        bufs_ptr: relocs.as_ptr() as _,
        cmds_ptr: cmds.as_ptr() as _,
        gather_data_ptr: buf.as_ptr() as _,
        syncobj_in: syncobj_in.map_or(0, u32::from),
        syncobj_out: syncobj_out.map_or(0, u32::from),
        syncpt: drm_tegra_submit_syncpt {
            id: sp_id,
            flags: 0,
            increments: sp_increments,
            value: 0,
        },
    };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_CHANNEL_SUBMIT, _>::new(&mut v),
        )?
    };
    Ok(v.syncpt.value)
}

pub fn syncpt_alloc(fd: &TegraDRM) -> io::Result<u32> {
    let mut v = drm_tegra_syncpoint_allocate { id: 0, padding: 0 };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_SYNCPOINT_ALLOCATE, _>::new(&mut v),
        )?
    }
    Ok(v.id)
}

pub fn syncpt_free(fd: &TegraDRM, id: u32) -> io::Result<()> {
    let mut v = drm_tegra_syncpoint_free { id, padding: 0 };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_SYNCPOINT_FREE, _>::new(&mut v),
        )
        .map_err(Into::into)
    }
}

pub fn syncpt_wait(fd: &TegraDRM, id: u32, threshold: u32, timeout_ns: i64) -> io::Result<u32> {
    let mut v = drm_tegra_syncpoint_wait {
        id,
        timeout_ns,
        threshold,
        value: 0,
        padding: 0,
    };
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<DRM_IOCTL_TEGRA_SYNCPOINT_WAIT, _>::new(&mut v),
        )?
    };
    Ok(v.value)
}
