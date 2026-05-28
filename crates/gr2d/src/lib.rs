#![no_std]

use colorspace::{BitsPerPixel, Encoding, Format, color::Color};
use embedded_graphics::primitives::Rectangle;
use gr2d_reg::{
    CLIPLEFTTOP, CLIPRIGHTBOT, CMDSEL, CTRL1, CTRLMAIN, DSTBA, DSTPS, DSTSIZE, DSTST, ROPFADE,
    SRCFGC, TILEMODE, TRIGGER0,
};
use host1x_buffer::{PixelBuffer, WritableBuffer};
use host1x_core::{
    cmd::{Mask, SetClass, WriteNoIncr},
    cmdseq::CommandSequence,
};
use tock_registers::fields::FieldValue;

pub trait Gr2d {
    unsafe fn exec_blocking<C: CommandSequence<gr2d_reg::Client>>(&self, cmdseq: C);

    /// Clear entire buffer to black color.
    fn clear<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(&self, buffer: &mut B) {
        self.clear_with_color(buffer, <<B::Format as Format>::Encoding as Encoding>::BLACK)
    }

    /// Clear entire buffer to the given color.
    fn clear_with_color<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(
        &self,
        buffer: &mut B,
        color: Color<<B::Format as Format>::Encoding>,
    ) {
        gr2d_fill_noclip(self, buffer, color);
    }

    /// Fill given region in a buffer with solid color.
    fn fill_rect<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(
        &self,
        buffer: &mut B,
        color: Color<<B::Format as Format>::Encoding>,
        rect: Rectangle,
    ) {
        gr2d_fill_clip(self, buffer, color, rect);
    }
}

fn gr2d_fill_noclip<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(
    gr2d: &(impl Gr2d + ?Sized),
    buffer: &mut B,
    color: Color<<B::Format as Format>::Encoding>,
) {
    let width = buffer.width();
    let height = buffer.height();
    let stride = buffer.stride();
    let bpp = <B::Format as Format>::Encoding::BPP;

    unsafe {
        gr2d.exec_blocking((
            SetClass::<_, _, gr2d_reg::Client, _>::new(),
            Mask::<2, 0, gr2d_reg::Client, _>::new((TRIGGER0::raw(0x3a), CMDSEL::OUTPUT::MEMORY)),
            Mask::<3, 0, gr2d_reg::Client, _>::new((
                CTRL1::CLIPC::DISABLE,
                CTRLMAIN::TURBOFILL::ENABLE
                    + CTRLMAIN::SRCSLD::ENABLE
                    + bpp_to_dstcd(bpp)
                    + CTRLMAIN::CMDT::BITBLT,
                ROPFADE::ROP::COPY,
            )),
            Mask::<2, 1, gr2d_reg::Client, _>::new((
                DSTBA::write(buffer),
                DSTST::DSTS.val(stride as _),
            )),
            WriteNoIncr::new([SRCFGC::SRCFGC.val(B::Format::color_to_gr2d_reg(color))]),
            WriteNoIncr::new([TILEMODE::zero()]),
            Mask::<2, 0, gr2d_reg::Client, _>::new((
                DSTSIZE::DSTWIDTH.val(width as _) + DSTSIZE::DSTHEIGHT.val(height as _),
                DSTPS::DSTX.val(0) + DSTPS::DSTY.val(0),
            )),
        ))
    };
}

fn gr2d_fill_clip<B: PixelBuffer + WritableBuffer<gr2d_reg::Client>>(
    gr2d: &(impl Gr2d + ?Sized),
    buffer: &mut B,
    color: Color<<B::Format as Format>::Encoding>,
    rect: Rectangle,
) {
    let width = buffer.width();
    let height = buffer.height();
    let stride = buffer.stride();
    let bpp = <B::Format as Format>::Encoding::BPP;

    unsafe {
        gr2d.exec_blocking((
            SetClass::<_, _, gr2d_reg::Client, _>::new(),
            Mask::<2, 0, gr2d_reg::Client, _>::new((TRIGGER0::raw(0x3a), CMDSEL::OUTPUT::MEMORY)),
            Mask::<5, 0, gr2d_reg::Client, _>::new((
                CTRL1::CLIPC::DRAW_INSIDE,
                // Can't use turbofill with clipping
                CTRLMAIN::TURBOFILL::DISABLE
                    + CTRLMAIN::SRCSLD::ENABLE
                    + bpp_to_dstcd(bpp)
                    + CTRLMAIN::CMDT::BITBLT,
                ROPFADE::ROP::COPY,
                CLIPLEFTTOP::CLIPL.val(rect.top_left.x as _)
                    + CLIPLEFTTOP::CLIPT.val(rect.top_left.y as _),
                CLIPRIGHTBOT::CLIPR.val(rect.top_left.x as u32 + rect.size.width as u32)
                    + CLIPRIGHTBOT::CLIPB.val(rect.top_left.y as u32 + rect.size.height as u32),
            )),
            Mask::<2, 1, gr2d_reg::Client, _>::new((
                DSTBA::write(buffer),
                DSTST::DSTS.val(stride as _),
            )),
            WriteNoIncr::new([SRCFGC::SRCFGC.val(B::Format::color_to_gr2d_reg(color))]),
            WriteNoIncr::new([TILEMODE::zero()]),
            Mask::<2, 0, gr2d_reg::Client, _>::new((
                DSTSIZE::DSTWIDTH.val(width as _) + DSTSIZE::DSTHEIGHT.val(height as _),
                DSTPS::DSTX.val(0) + DSTPS::DSTY.val(0),
            )),
        ))
    };
}

fn bpp_to_dstcd(bpp: BitsPerPixel) -> FieldValue<u32, CTRLMAIN::Register> {
    match bpp {
        BitsPerPixel::BPP8 => CTRLMAIN::DSTCD::BPP8,
        BitsPerPixel::BPP16 => CTRLMAIN::DSTCD::BPP16,
        BitsPerPixel::BPP32 => CTRLMAIN::DSTCD::BPP32,
    }
}
