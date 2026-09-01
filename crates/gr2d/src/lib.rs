#![no_std]

pub mod fill;
pub mod format;
mod util;

use colorspace::{Encoding, Format, color::Color};
use embedded_graphics::primitives::Rectangle;
use host1x_buffer::{PixelBuffer, WritableBuffer};

use crate::format::Gr2dFormat;

/// Clear entire buffer to black color.
pub fn clear<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(
    buffer: B,
) -> fill::Fill<B, fill::NoClip>
where
    B::Format: Gr2dFormat,
{
    clear_with_color(buffer, <<B::Format as Format>::Encoding as Encoding>::BLACK)
}

/// Clear entire buffer to the given color.
pub fn clear_with_color<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(
    buffer: B,
    color: Color<<B::Format as Format>::Encoding>,
) -> fill::Fill<B, fill::NoClip>
where
    B::Format: Gr2dFormat,
{
    fill::Fill::new(buffer, fill::NoClip, color)
}

/// Fill given region in a buffer with solid color.
pub fn fill_rect<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(
    buffer: B,
    color: Color<<B::Format as Format>::Encoding>,
    rect: Rectangle,
) -> fill::Fill<B, fill::Clip>
where
    B::Format: Gr2dFormat,
{
    fill::Fill::new(buffer, fill::Clip(rect), color)
}
