use std::{marker::PhantomData, num::NonZeroU32};

use colorspace::Format;
use host1x_buffer::PixelDesc;

pub(crate) mod sealed {
    use std::num::NonZeroU32;

    pub trait SealedBufferType {
        type Desc: host1x_buffer::Layout + Copy;

        /// Description of a buffer of this type, backed by `size` bytes of memory.
        fn desc(&self, size: NonZeroU32) -> Self::Desc;
    }
}
pub trait BufferType: Copy + sealed::SealedBufferType {}

#[derive(Debug, Clone, Copy)]
pub struct PixelBuffer<F: Format> {
    pub(crate) width: NonZeroU32,
    pub(crate) height: NonZeroU32,
    pub(crate) _format: PhantomData<F>,
}
impl<F: Format> sealed::SealedBufferType for PixelBuffer<F> {
    type Desc = PixelDesc<F>;

    fn desc(&self, size: NonZeroU32) -> PixelDesc<F> {
        let stride = size.get() / self.height.get();
        PixelDesc::new(size.get(), self.width.get(), self.height.get(), stride)
    }
}
impl<F: Format> BufferType for PixelBuffer<F> {}
