#![no_std]

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
