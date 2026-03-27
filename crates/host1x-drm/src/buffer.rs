use std::{marker::PhantomData, num::NonZeroU32};

use colorspace::Format;

mod sealed {
    pub trait SealedBufferType {}
}
pub trait BufferType: Copy + sealed::SealedBufferType {}

#[derive(Debug)]
pub struct PixelBuffer<F: Format> {
    pub(crate) width: NonZeroU32,
    pub(crate) height: NonZeroU32,
    pub(crate) _format: PhantomData<F>,
}
impl<F: Format> Clone for PixelBuffer<F> {
    fn clone(&self) -> Self {
        Self {
            width: self.width.clone(),
            height: self.height.clone(),
            _format: PhantomData,
        }
    }
}
impl<F: Format> Copy for PixelBuffer<F> {}
impl<F: Format> sealed::SealedBufferType for PixelBuffer<F> {}
impl<F: Format> BufferType for PixelBuffer<F> {}
