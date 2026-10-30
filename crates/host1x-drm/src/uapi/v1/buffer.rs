use std::{
    io,
    marker::PhantomData,
    mem::{ManuallyDrop, forget},
    num::NonZeroU32,
    ops::{Deref, DerefMut},
    os::fd::{AsFd, AsRawFd},
    ptr::{self, NonNull},
    slice,
};

use colorspace::{Encoding, Format};
use host1x_buffer::access::{ReadOnly, ReadWrite, Readable};
use rustix::mm::{MapFlags, mmap, munmap};
use tracing::field;

use crate::{
    buffer::{BufferType, PixelBuffer},
    device::TegraDRM,
    mapping::{Mapped, MappingMode, Unmapped},
};

use super::ioctl;

pub type Buffer<T> = GenericBuffer<Unmapped, T>;
pub type RoBuffer<T> = GenericBuffer<ReadOnly, T>;
pub type RwBuffer<T> = GenericBuffer<ReadWrite, T>;

#[derive(Debug)]
pub struct GenericBuffer<MM: MappingMode, T: BufferType> {
    pub(crate) dev: TegraDRM,
    pub(crate) handle: u32,
    /// Size of backing memory in bytes.
    size: NonZeroU32,
    ptr: ManuallyDrop<MM::Handle<NonNull<u8>>>,
    /// Type-specific additional data
    typ_dat: T,
}
impl<MM: MappingMode, F: Format> GenericBuffer<MM, PixelBuffer<F>> {
    // TODO: maybe make this private? Then buffers would be created through PixelBuffer, CommandBuffer,
    // or other specialized classes.

    /// Create new GEM buffer.
    #[instrument(skip_all, fields(
        dev = dev.as_fd().as_raw_fd(),
    ))]
    pub fn new(dev: &TegraDRM, width: NonZeroU32, height: NonZeroU32) -> io::Result<Self> {
        struct DropGuard<'a> {
            dev: &'a TegraDRM,
            gem: u32,
        }
        impl Drop for DropGuard<'_> {
            fn drop(&mut self) {
                let r = drm_ffi::gem::close(self.dev.as_fd(), self.gem);
                #[cfg(debug_assertions)]
                r.unwrap();
            }
        }

        let stride = width.get() * F::Encoding::BPP.as_u8() as u32 / 8;

        let size = NonZeroU32::new(stride * height.get()).expect("size should be nonzero");
        let handle = ioctl::gem_create(
            dev,
            size.get() as _,
            ioctl::drm_tegra_gem_create_flags::empty(),
        )
        .inspect_err(|error| error!(error = field::display(error), "GEM buffer create fail"))?;
        debug!(handle, "created GEM buffer");
        let guard = DropGuard { dev, gem: handle };
        let ptr = unsafe { create_gem_mapping::<MM>(dev, handle, size)? };
        forget(guard);

        Ok(Self {
            dev: dev.clone(),
            handle,
            size,
            ptr: ManuallyDrop::new(ptr),
            typ_dat: PixelBuffer {
                width,
                height,
                _format: PhantomData,
            },
        })
    }
}
impl<T: BufferType> GenericBuffer<Unmapped, T> {
    /// Map into address space of current program in read-only mode.
    pub fn mmap_ro(self) -> ::std::result::Result<RoBuffer<T>, (io::Error, Self)> {
        match unsafe { create_gem_mapping::<ReadOnly>(&self.dev, self.handle, self.size) } {
            Ok(ptr) => {
                let this = ManuallyDrop::new(self);
                Ok(GenericBuffer {
                    dev: this.dev.clone(),
                    handle: this.handle,
                    size: this.size,
                    ptr: ManuallyDrop::new(ptr),
                    typ_dat: this.typ_dat,
                })
            }
            Err(error) => Err((error, self)),
        }
    }

    /// Map into address space of current program in read-write mode.
    pub fn mmap_rw(self) -> ::std::result::Result<RwBuffer<T>, (io::Error, Self)> {
        match unsafe { create_gem_mapping::<ReadWrite>(&self.dev, self.handle, self.size) } {
            Ok(ptr) => {
                let this = ManuallyDrop::new(self);
                Ok(GenericBuffer {
                    dev: this.dev.clone(),
                    handle: this.handle,
                    size: this.size,
                    ptr: ManuallyDrop::new(ptr),
                    typ_dat: this.typ_dat,
                })
            }
            Err(error) => Err((error, self)),
        }
    }

    // TODO: support unmapping and remapping
}
impl<MM: Mapped + Readable, T: BufferType> Deref for GenericBuffer<MM, T> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl<T: BufferType> DerefMut for GenericBuffer<ReadWrite, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}
impl<MM: Mapped + Readable, T: BufferType> AsRef<[u8]> for GenericBuffer<MM, T> {
    fn as_ref(&self) -> &[u8] {
        // Safety:
        // - constructor ensures self.ptr is valid
        unsafe {
            slice::from_raw_parts(
                MM::get_handle_ref(&self.ptr).as_ptr() as *const u8,
                self.size.get().try_into().unwrap(),
            )
        }
    }
}
impl<T: BufferType> AsMut<[u8]> for GenericBuffer<ReadWrite, T> {
    fn as_mut(&mut self) -> &mut [u8] {
        // Safety:
        // - constructor ensures self.ptr is valid
        // - DerefMut is implemented only for ReadWrite, preventing mutable references to ro mapping
        // - we require self to be mutable borrow by restricting `BM: AsSyntheticHandleMut`
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size.get().try_into().unwrap()) }
    }
}
// We don't support write-only (would it even have any real use?). In WO we can't expose [u8] slice directly because that implies
// it is readable.
// Rust currently doesn't have write-only references so we would have use some type of write-only
// wrapper, perhabs something like https://docs.rs/write_ref/latest/write_ref/ or https://github.com/gfx-rs/wgpu/blob/81eca17db8a92288b866f3baf93498d7b2225de7/wgpu-types/src/write_only.rs
// would do.
impl<MM: MappingMode, T: BufferType> host1x_buffer::Layout for GenericBuffer<MM, T> {
    fn size(&self) -> u32 {
        self.size.get()
    }
}
impl<MM: MappingMode, T: BufferType> host1x_buffer::Buffer for GenericBuffer<MM, T> {
    type Desc = T::Desc;

    fn desc(&self) -> T::Desc {
        self.typ_dat.desc(self.size)
    }
}
impl<MM: MappingMode, F: Format> host1x_buffer::PixelLayout for GenericBuffer<MM, PixelBuffer<F>> {
    type Format = F;

    fn width(&self) -> u32 {
        self.typ_dat.width.get()
    }

    fn height(&self) -> u32 {
        self.typ_dat.height.get()
    }

    fn stride(&self) -> u32 {
        self.size.get() / self.typ_dat.height.get()
    }
}
impl<MM: MappingMode, T: BufferType> Drop for GenericBuffer<MM, T> {
    fn drop(&mut self) {
        let span = debug_span!(
            "drop",
            dev.fd = self.dev.as_fd().as_raw_fd(),
            buffer.id = self.handle,
        );
        let _guard = span.enter();

        struct BufDestroyGuard<'a> {
            dev: &'a TegraDRM,
            gem: u32,
        }
        impl Drop for BufDestroyGuard<'_> {
            fn drop(&mut self) {
                debug!("destroying buffer");
                let r = drm_ffi::gem::close(self.dev.as_fd(), self.gem);
                #[cfg(debug_assertions)]
                r.unwrap();
            }
        }

        let _buf_destroy = BufDestroyGuard {
            dev: &self.dev,
            gem: self.handle,
        };
        unsafe {
            MM::destroy_mapping(
                ManuallyDrop::take(&mut self.ptr),
                || (),
                |ptr| {
                    debug!(
                        "unmapping {:p}-{:p}",
                        ptr,
                        ptr.as_ptr()
                            .wrapping_add((self.size.get() - 1).try_into().unwrap())
                    );

                    let r = munmap(ptr.as_ptr() as *mut _, self.size.get().try_into().unwrap());
                    #[cfg(debug_assertions)]
                    r.unwrap();
                },
            )
        };
    }
}
unsafe fn create_gem_mapping<MM: MappingMode>(
    dev: &TegraDRM,
    gem: u32,
    size: NonZeroU32,
) -> io::Result<MM::Handle<NonNull<u8>>> {
    MM::create_cpu_mapping(|prot| {
        let span = debug_span!(
            "create_gem_mapping",
            dev.fd = dev.as_fd().as_raw_fd(),
            buffer.id = gem,
            buffer.size = size,
            mode = MM::MODE_NAME,
        );
        let _guard = span.enter();
        let size = size.get().try_into().unwrap();

        ioctl::gem_mmap(dev, gem).and_then(|offset| {
            debug!(offset, "mmap GEM");
            unsafe {
                mmap(ptr::null_mut(), size, prot, MapFlags::SHARED, dev, offset)
                    .inspect(|ptr| {
                        debug!(
                            "mapped to {:p}-{:p}",
                            ptr,
                            (*ptr as *mut u8).wrapping_add(size - 1)
                        )
                    })
                    .map_err(io::Error::from)
                    .map(|p| NonNull::new_unchecked(p as *mut u8))
            }
        })
    })
}
