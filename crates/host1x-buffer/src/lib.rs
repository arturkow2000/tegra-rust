#![no_std]

use core::marker::PhantomData;

use colorspace::Format;

pub mod access;
pub mod fmt;

/// A buffer that is mapped into address space of given Host1x client.
///
/// `'id` is a brand identifying the job (see `host1x_core::job`) that owns
/// the buffer for the duration of GPU work. Buffers are never branded by
/// themselves, only by GPU-side handles created by the job.
pub trait MappedBuffer<'id, C: host1x_client::Client>: Buffer {
    /// Opaque identifier for the mapping. Used for relocations.
    fn mapping_id(&self) -> u32;
}

/// A buffer that is readable by given Host1x client.
pub trait ReadableBuffer<'id, C: host1x_client::Client>: MappedBuffer<'id, C> {}

/// A buffer that is writable by given Host1x client.
pub trait WritableBuffer<'id, C: host1x_client::Client>: MappedBuffer<'id, C> {}

/// Shape of a buffer: how its content is organized in memory.
///
/// Implemented both by buffers (and handles to them) and by plain layout
/// descriptions such as [`LinearDesc`], which are not buffers themselves.
pub trait Layout {
    /// Buffer size in bytes.
    fn size(&self) -> u32;
}

/// Shape of a buffer holding pixel data. Contrary to generic layout has
/// clearly defined width, height and format.
pub trait PixelLayout: Layout {
    type Format: Format;

    /// Buffer width in pixels.
    fn width(&self) -> u32;

    /// Buffer height in pixels.
    fn height(&self) -> u32;

    /// Buffer stride (pitch) in bytes.
    fn stride(&self) -> u32;
}

/// A generic buffer of unspecified type.
pub trait Buffer: Layout {
    /// Layout description detached from the memory backing the buffer.
    type Desc: Layout + Copy;

    /// Describe layout of the buffer.
    fn desc(&self) -> Self::Desc;
}

/// Buffer for holding pixel data.
///
/// Implemented for every [`Buffer`] with [`PixelLayout`].
pub trait PixelBuffer: Buffer + PixelLayout {}
impl<B: Buffer + PixelLayout + ?Sized> PixelBuffer for B {}

// Shared references allow tasks to read from a buffer without taking ownership.
impl<L: Layout + ?Sized> Layout for &L {
    fn size(&self) -> u32 {
        (**self).size()
    }
}
impl<L: PixelLayout + ?Sized> PixelLayout for &L {
    type Format = L::Format;

    fn width(&self) -> u32 {
        (**self).width()
    }

    fn height(&self) -> u32 {
        (**self).height()
    }

    fn stride(&self) -> u32 {
        (**self).stride()
    }
}
impl<B: Buffer + ?Sized> Buffer for &B {
    type Desc = B::Desc;

    fn desc(&self) -> Self::Desc {
        (**self).desc()
    }
}
impl<'id, C: host1x_client::Client, B: MappedBuffer<'id, C> + ?Sized> MappedBuffer<'id, C> for &B {
    fn mapping_id(&self) -> u32 {
        (**self).mapping_id()
    }
}
impl<'id, C: host1x_client::Client, B: ReadableBuffer<'id, C> + ?Sized> ReadableBuffer<'id, C>
    for &B
{
}

/// Description of a buffer holding plain bytes.
#[derive(Debug, Clone, Copy)]
pub struct LinearDesc {
    size: u32,
}
impl LinearDesc {
    pub fn new(size: u32) -> Self {
        Self { size }
    }
}
impl Layout for LinearDesc {
    fn size(&self) -> u32 {
        self.size
    }
}

/// Description of a pixel buffer.
#[derive(Debug, Clone, Copy)]
pub struct PixelDesc<F: Format> {
    size: u32,
    width: u32,
    height: u32,
    stride: u32,
    _format: PhantomData<F>,
}
impl<F: Format> PixelDesc<F> {
    pub fn new(size: u32, width: u32, height: u32, stride: u32) -> Self {
        Self {
            size,
            width,
            height,
            stride,
            _format: PhantomData,
        }
    }
}
impl<F: Format> Layout for PixelDesc<F> {
    fn size(&self) -> u32 {
        self.size
    }
}
impl<F: Format> PixelLayout for PixelDesc<F> {
    type Format = F;

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn stride(&self) -> u32 {
        self.stride
    }
}
