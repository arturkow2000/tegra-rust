#![no_std]

use core::marker::PhantomData;

use colorspace::Format;

pub mod access;
pub mod fmt;

/// A buffer that is mapped into address space of given Host1x client.
pub trait MappedBuffer<C: host1x_client::Client>: Buffer {
    /// Opaque identifier for the mapping. Used for relocations.
    fn mapping_id(&self) -> u32;
}

/// A buffer that is readable by given Host1x client.
pub trait ReadableBuffer<C: host1x_client::Client>: MappedBuffer<C> {}

/// A buffer that is writable by given Host1x client.
pub trait WritableBuffer<C: host1x_client::Client>: MappedBuffer<C> {}

/// A generic buffer of unspecified type.
pub trait Buffer {
    /// Buffer size in bytes.
    fn size(&self) -> u32;
}

/// Buffer for holding pixel data. Contrary to standard buffer has clearly
/// defined width, height and format.
pub trait PixelBuffer: Buffer {
    type Format: Format;

    /// Buffer width in pixels.
    fn width(&self) -> u32;

    /// Buffer height in pixels.
    fn height(&self) -> u32;

    /// Buffer stride (pitch) in bytes.
    fn stride(&self) -> u32;
}

/// Layout of a pixel buffer, detached from the memory backing it.
#[derive(Debug, Clone, Copy)]
pub struct PixelLayout<F: Format> {
    size: u32,
    width: u32,
    height: u32,
    stride: u32,
    _format: PhantomData<F>,
}
impl<F: Format> PixelLayout<F> {
    /// Capture layout of given pixel buffer.
    pub fn of(buf: &impl PixelBuffer<Format = F>) -> Self {
        Self {
            size: buf.size(),
            width: buf.width(),
            height: buf.height(),
            stride: buf.stride(),
            _format: PhantomData,
        }
    }
}
impl<F: Format> Buffer for PixelLayout<F> {
    fn size(&self) -> u32 {
        self.size
    }
}
impl<F: Format> PixelBuffer for PixelLayout<F> {
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
