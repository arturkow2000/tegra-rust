//! Test helpers for exercising GR2D on real hardware.

pub mod ctx;
pub mod util;

use std::ops::Deref;

use colorspace::{Encoding, Format, Rgb, color::Color};
use gr2d::format::Gr2dFormat;

/// Alias for RGB format.
///
/// Tests should bound on this instead on [`Format`] directly or will be rejected
/// by [`tegra_test_macros::test`] macro.
pub trait RgbFormat: Format<Encoding: Rgb<R = u8, G = u8, B = u8>> + Gr2dFormat {}
impl<T: Format<Encoding: Rgb<R = u8, G = u8, B = u8>> + Gr2dFormat> RgbFormat for T {}

/// Helper macro that executes GR2D pipeline on fresh buffer of given width,
/// height and format. On job completion it returns the buffer to the caller.
#[macro_export]
macro_rules! exec {
    ($ctx:expr, $format:ty, $width:expr, $height:expr, $pipeline:expr $(,)?) => {{
        let ctx: &$crate::ctx::Gr2d = $ctx;
        let queue = ctx.queue();
        let buffer = ctx.buffer::<$format>(&queue, $width, $height);
        let job = host1x_core::job!(buffer, $pipeline);
        ctx.exec(&queue, job).await
    }};
}

// FIXME: raw versions won't work for planar buffers, make them available on linear exclusively.
/// Helper methods for accessing pixel buffers.
pub trait PixelBufferExt: host1x_buffer::PixelBuffer + Deref<Target = [u8]> {
    fn bits_per_pixel(&self) -> usize {
        <Self::Format as Format>::Encoding::BPP.as_u8() as usize
    }

    /// Bytes per pixel
    fn bytes_per_pixel(&self) -> usize {
        self.bits_per_pixel() / 8
    }

    /// Get raw pixel.
    fn pixel_raw(&self, x: u32, y: u32) -> &[u8] {
        let bpp = self.bytes_per_pixel();
        let off = y as usize * self.stride() as usize + x as usize * bpp;
        // TODO: introduce get_raw_pixel into colorspace instead of duplicating
        // this logic here.
        self.get(off..off + bpp).expect("pixel within bounds")
    }

    /// Call given closure for each pixel, raw version.
    fn for_each_raw_pixel(&self, mut f: impl FnMut(u32, u32, &[u8])) {
        for y in 0..self.height() {
            for x in 0..self.width() {
                f(x, y, self.pixel_raw(x, y));
            }
        }
    }

    /// Get pixel decoded to a [`Color`].
    fn pixel(&self, x: u32, y: u32) -> Color<<Self::Format as Format>::Encoding> {
        let bpp = <<Self::Format as Format>::Encoding as Encoding>::BPP;
        <Self::Format as Format>::get_pixel(x, y, &[(self.stride(), &self[..])], bpp)
            .expect("pixel within bounds")
    }

    /// Call given closure for each pixel.
    fn for_each_pixel(
        &self,
        mut f: impl FnMut(u32, u32, Color<<Self::Format as Format>::Encoding>),
    ) {
        for y in 0..self.height() {
            for x in 0..self.width() {
                f(x, y, self.pixel(x, y));
            }
        }
    }
}
impl<B: host1x_buffer::PixelBuffer + Deref<Target = [u8]> + ?Sized> PixelBufferExt for B {}
