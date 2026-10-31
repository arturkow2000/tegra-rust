#![no_std]

pub mod fill;
pub mod format;
mod util;

use colorspace::{Encoding, Format, color::Color};
use embedded_graphics::primitives::Rectangle;
use host1x_buffer::{PixelBuffer, WritableBuffer};

use crate::format::Gr2dFormat;

/// Clear entire buffer to black color.
pub fn clear<'id, B: PixelBuffer + WritableBuffer<'id, gr2d_reg::Client>>(
    buffer: B,
) -> fill::Fill<B, fill::NoClip>
where
    B::Format: Gr2dFormat,
{
    clear_with_color(buffer, <<B::Format as Format>::Encoding as Encoding>::BLACK)
}

/// Clear entire buffer to the given color.
///
/// The color is written as is, including its alpha channel, without blending.
pub fn clear_with_color<'id, B: PixelBuffer + WritableBuffer<'id, gr2d_reg::Client>>(
    buffer: B,
    color: Color<<B::Format as Format>::Encoding>,
) -> fill::Fill<B, fill::NoClip>
where
    B::Format: Gr2dFormat,
{
    fill::Fill::new_raw(buffer, fill::NoClip, color)
}

/// Fill given region in a buffer with solid color.
///
/// `color` must have the same RGB channels as the buffer, but its alpha channel
/// is independent of the buffer's (see [`fill::FillColor`]). Colors without
/// alpha or with fully opaque alpha replace the pixels, others are blended onto
/// them.
pub fn fill_rect<'id, B: PixelBuffer + WritableBuffer<'id, gr2d_reg::Client>>(
    buffer: B,
    color: impl fill::FillColor<<B::Format as Format>::Encoding>,
    rect: Rectangle,
) -> fill::Fill<B, fill::Clip>
where
    B::Format: Gr2dFormat,
{
    fill::Fill::new(buffer, fill::Clip(rect), color)
}
