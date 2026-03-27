use std::{
    io,
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Deref, DerefMut},
    os::fd::{AsFd, AsRawFd as _, BorrowedFd},
    ptr::NonNull,
};

use colorspace::Colorspace;
use libhandle::synth::{
    AsSyntheticHandle, AsSyntheticHandleMut, BorrowedSyntheticHandle, BorrowedSyntheticHandleMut,
    HandleDestroy, OwnedSyntheticHandle,
};
use reborrow::{Reborrow, ReborrowMut};

use crate::{
    device::TegraDRM,
    mapping::{MappingMode, ReadOnly, ReadWrite, Readable, Unmapped},
};

use super::buffer::Buffer;

#[derive(Debug)]
pub struct PixelBuffer<
    DH: AsFd,
    BH: AsSyntheticHandle<u32> + HandleDestroy,
    BM: AsSyntheticHandle<NonNull<u8>> + HandleDestroy,
    MM: MappingMode,
    CS: Colorspace,
> {
    inner: Buffer<DH, BH, BM, MM>,
    width: u32,
    stride: u32,
    _colorspace: PhantomData<CS>,
}
impl<DH: AsFd, CS: Colorspace>
    PixelBuffer<DH, OwnedSyntheticHandle<u32>, OwnedSyntheticHandle<NonNull<u8>>, Unmapped, CS>
{
    /// Create new GEM pixel buffer.
    #[instrument(skip_all, fields(
        dev = dev.rb().as_fd().as_raw_fd(),
        width = width,
        height = height,
        stride = stride,
    ))]
    pub fn new(dev: TegraDRM<DH>, width: u32, height: u32, stride: u32) -> io::Result<Self> {
        let size = height * stride;
        Ok(Self {
            inner: Buffer::new(dev, size)?,
            width,
            stride,
            _colorspace: PhantomData,
        })
    }
}
impl<
    DH: AsFd,
    BH: AsSyntheticHandleMut<u32> + HandleDestroy,
    BM: AsSyntheticHandleMut<NonNull<u8>> + HandleDestroy,
    CS: Colorspace,
> PixelBuffer<DH, BH, BM, Unmapped, CS>
{
    #[allow(clippy::type_complexity)]
    /// Map into address space of current program in read-only mode.
    pub fn try_mmap(
        self,
    ) -> ::std::result::Result<
        PixelBuffer<DH, BH, OwnedSyntheticHandle<NonNull<u8>>, ReadOnly, CS>,
        (io::Error, Self),
    > {
        Ok(PixelBuffer {
            inner: self.inner.try_mmap().map_err(|(err, buf)| {
                (
                    err,
                    Self {
                        inner: buf,
                        width: self.width,
                        stride: self.stride,
                        _colorspace: PhantomData,
                    },
                )
            })?,
            width: self.width,
            stride: self.stride,
            _colorspace: PhantomData,
        })
    }

    #[allow(clippy::type_complexity)]
    /// Map into address space of current program in read-write mode.
    pub fn try_mmap_mut(
        self,
    ) -> ::std::result::Result<
        PixelBuffer<DH, BH, OwnedSyntheticHandle<NonNull<u8>>, ReadWrite, CS>,
        (io::Error, Self),
    > {
        Ok(PixelBuffer {
            inner: self.inner.try_mmap_mut().map_err(|(err, buf)| {
                (
                    err,
                    Self {
                        inner: buf,
                        width: self.width,
                        stride: self.stride,
                        _colorspace: PhantomData,
                    },
                )
            })?,
            width: self.width,
            stride: self.stride,
            _colorspace: PhantomData,
        })
    }
}
impl<
    'short,
    DH: AsFd,
    BH: AsSyntheticHandle<u32> + HandleDestroy,
    BM: AsSyntheticHandle<NonNull<u8>> + HandleDestroy,
    MM: MappingMode,
    CS: Colorspace,
> Reborrow<'short> for PixelBuffer<DH, BH, BM, MM, CS>
{
    type Target = PixelBuffer<
        BorrowedFd<'short>,
        BorrowedSyntheticHandle<'short, u32>,
        BorrowedSyntheticHandle<'short, NonNull<u8>>,
        MM,
        CS,
    >;

    fn rb(&'short self) -> Self::Target {
        PixelBuffer {
            inner: self.inner.rb(),
            width: self.width,
            stride: self.stride,
            _colorspace: PhantomData,
        }
    }
}
impl<
    'short,
    DH: AsFd,
    BH: AsSyntheticHandleMut<u32> + HandleDestroy,
    BM: AsSyntheticHandleMut<NonNull<u8>> + HandleDestroy,
    MM: MappingMode,
    CS: Colorspace,
> ReborrowMut<'short> for PixelBuffer<DH, BH, BM, MM, CS>
{
    type Target = PixelBuffer<
        BorrowedFd<'short>,
        BorrowedSyntheticHandleMut<'short, u32>,
        BorrowedSyntheticHandleMut<'short, NonNull<u8>>,
        MM,
        CS,
    >;

    fn rb_mut(&'short mut self) -> Self::Target {
        PixelBuffer {
            inner: self.inner.rb_mut(),
            width: self.width,
            stride: self.stride,
            _colorspace: PhantomData,
        }
    }
}
impl<
    'short,
    DH: AsFd,
    BH: AsSyntheticHandle<u32> + HandleDestroy,
    BM: AsSyntheticHandle<NonNull<u8>> + HandleDestroy,
    MM: MappingMode,
    CS: Colorspace,
> drm::buffer::Buffer for PixelBuffer<DH, BH, BM, MM, CS>
{
    fn size(&self) -> (u32, u32) {
        (self.width, self.inner.size() / self.stride)
    }

    fn format(&self) -> drm::buffer::DrmFourcc {
        CS::DRM_FCC
    }

    fn pitch(&self) -> u32 {
        self.stride
    }

    fn handle(&self) -> drm::buffer::Handle {
        NonZeroU32::new(self.inner.handle.as_synthetic_handle().get())
            .unwrap()
            .into()
    }
}
impl<
    DH: AsFd,
    BH: AsSyntheticHandle<u32> + HandleDestroy,
    BM: AsSyntheticHandle<NonNull<u8>> + HandleDestroy,
    MM: Readable,
    CS: Colorspace,
> Deref for PixelBuffer<DH, BH, BM, MM, CS>
{
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
impl<
    DH: AsFd,
    BH: AsSyntheticHandleMut<u32> + HandleDestroy,
    BM: AsSyntheticHandleMut<NonNull<u8>> + HandleDestroy,
    CS: Colorspace,
> DerefMut for PixelBuffer<DH, BH, BM, ReadWrite, CS>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}
