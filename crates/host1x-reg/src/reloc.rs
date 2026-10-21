//! Relocation support for Host1x registers.

use core::marker::PhantomData;

use getset::CopyGetters;
pub use host1x_buffer::access::{ReadOnly, ReadWrite, WriteOnly};
use host1x_buffer::{ReadableBuffer, WritableBuffer};
use host1x_client::Client;
use tock_registers::RegisterLongName;

/// A relocatable memory pointer.
///
/// Some registers hold absolute pointers to memory and can't be written directly
/// by the userspace. Those are written empty to the command stream and additional
/// relocation data is created, that during command submission is handled by
/// kernel mode driver to compute absolute memory addresses to be programmed into GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, CopyGetters)]
pub struct Relocatable<'a, R: RegisterLongName, M, C: Client> {
    #[get_copy = "pub"]
    mapping: u32,

    #[get_copy = "pub"]
    buf_offset: u64,

    #[get_copy = "pub"]
    shift: u32,

    _reg: PhantomData<R>,
    _lifetime: PhantomData<&'a ()>,
    _mode: PhantomData<M>,
    _client: PhantomData<C>,
}

impl<R: RegisterLongName, M, C: Client> Relocatable<'static, R, M, C> {
    /// Create a relocatable address using raw mapping ID.
    ///
    /// # Safety
    /// This bypasses Rust ownership model in reference to the buffer (relocation target).
    /// Caller must ensure that [`Relocatable`] doesn't outlive the mapping and that
    /// Rust's aliasing rules are respected.
    pub const unsafe fn for_raw_mapping(mapping: u32, buf_offset: u64, shift: u32) -> Self {
        Self {
            mapping,
            buf_offset,
            shift,
            _reg: PhantomData,
            _lifetime: PhantomData,
            _mode: PhantomData,
            _client: PhantomData,
        }
    }
}

impl<'a, R: RegisterLongName, C: Client> Relocatable<'a, R, ReadOnly, C> {
    /// Create relocatable address for reading from the buffer.
    pub fn read(buf: &'a impl ReadableBuffer<C>) -> Self {
        Self {
            mapping: buf.mapping_id(),
            buf_offset: 0,
            shift: 0,
            _reg: PhantomData,
            _lifetime: PhantomData,
            _mode: PhantomData,
            _client: PhantomData,
        }
    }
}

impl<'a, R: RegisterLongName, C: Client> Relocatable<'a, R, WriteOnly, C> {
    /// Create relocatable address for writing to the buffer.
    pub fn write(buf: &'a mut impl WritableBuffer<C>) -> Self {
        Self {
            mapping: buf.mapping_id(),
            buf_offset: 0,
            shift: 0,
            _reg: PhantomData,
            _lifetime: PhantomData,
            _mode: PhantomData,
            _client: PhantomData,
        }
    }
}

impl<'a, R: RegisterLongName, C: Client> Relocatable<'a, R, ReadWrite, C> {
    // TODO: support offsets in safe API.

    /// Create relocatable address for reading and writing the buffer.
    pub fn new(buf: &'a mut (impl ReadableBuffer<C> + WritableBuffer<C>)) -> Self {
        Self {
            mapping: buf.mapping_id(),
            buf_offset: 0,
            shift: 0,
            _reg: PhantomData,
            _lifetime: PhantomData,
            _mode: PhantomData,
            _client: PhantomData,
        }
    }
}

/// Relocation metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Relocation {
    /// 32-bit word offset into command stream where to patch-in computed address.
    pub patch_offset: u32,

    /// Opaque mapping ID of relocation target. This is returned by the kernel when
    /// mapping buffer into Host1x address space.
    pub mapping: u32,

    /// Byte offset relative to target buffer base.
    pub buf_offset: u64,

    /// How many bits to shift right the address before patching it into command stream.
    // TODO: What's the use of this feature?
    pub shift: u32,
}
