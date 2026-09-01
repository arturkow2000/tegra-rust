use colorspace::{Encoding, Format, color::Color};
use embedded_graphics::primitives::Rectangle;
use gr2d_reg::{
    CLIPLEFTTOP, CLIPRIGHTBOT, CMDSEL, CTRL1, CTRLMAIN, DSTBA, DSTPS, DSTSIZE, DSTST, ROPFADE,
    SRCFGC, TILEMODE, TRIGGER0,
};
use host1x_buffer::{PixelBuffer, WritableBuffer};
use host1x_core::{
    cmd::{Mask, SetClass, WriteNoIncr},
    cmdseq::CommandSequence,
    task::Task,
};

use crate::{format::Gr2dFormat, util::bpp_to_dstcd};

mod p {
    use colorspace::Format;
    use host1x_core::cmdseq::CommandSequence;

    use crate::format::Gr2dFormat;

    pub trait ClipMode {
        fn cmdseq<F: Format + Gr2dFormat>(&self) -> impl CommandSequence<gr2d_reg::Client>;
    }
}

pub trait ClipMode: p::ClipMode {}

pub struct Clip(pub Rectangle);
impl p::ClipMode for Clip {
    fn cmdseq<F: Format + Gr2dFormat>(
        &self,
    ) -> impl host1x_core::cmdseq::CommandSequence<gr2d_reg::Client> {
        let bpp = <F as Format>::Encoding::BPP;

        (Mask::<6, 0, gr2d_reg::Client, _>::new((
            F::SIFMT + F::DIFMT,
            CTRL1::CLIPC::DRAW_INSIDE,
            // Can't use turbofill with clipping
            CTRLMAIN::TURBOFILL::DISABLE
                + CTRLMAIN::SRCSLD::ENABLE
                + bpp_to_dstcd(bpp)
                + CTRLMAIN::CMDT::BITBLT,
            ROPFADE::ROP::COPY,
            CLIPLEFTTOP::CLIPL.val(self.0.top_left.x as _)
                + CLIPLEFTTOP::CLIPT.val(self.0.top_left.y as _),
            CLIPRIGHTBOT::CLIPR.val(self.0.top_left.x as u32 + self.0.size.width as u32)
                + CLIPRIGHTBOT::CLIPB.val(self.0.top_left.y as u32 + self.0.size.height as u32),
        )),)
    }
}
impl ClipMode for Clip {}

pub struct NoClip;
impl p::ClipMode for NoClip {
    fn cmdseq<F: Format + Gr2dFormat>(
        &self,
    ) -> impl host1x_core::cmdseq::CommandSequence<gr2d_reg::Client> {
        let bpp = <F as Format>::Encoding::BPP;

        (Mask::<4, 0, gr2d_reg::Client, _>::new((
            F::SIFMT + F::DIFMT,
            CTRL1::CLIPC::DISABLE,
            CTRLMAIN::TURBOFILL::ENABLE
                + CTRLMAIN::SRCSLD::ENABLE
                + bpp_to_dstcd(bpp)
                + CTRLMAIN::CMDT::BITBLT,
            ROPFADE::ROP::COPY,
        )),)
    }
}
impl ClipMode for NoClip {}

pub struct Fill<Buffer: PixelBuffer, Clip> {
    buffer: Buffer,
    clip: Clip,
    color: Color<<Buffer::Format as Format>::Encoding>,
}
impl<Buffer: PixelBuffer + WritableBuffer<gr2d_reg::Client>, Clip: ClipMode> Fill<Buffer, Clip> {
    pub(crate) fn new(
        buffer: Buffer,
        clip: Clip,
        color: Color<<Buffer::Format as Format>::Encoding>,
    ) -> Self {
        Self {
            buffer,
            clip,
            color,
        }
    }
}
unsafe impl<Buffer: PixelBuffer + WritableBuffer<gr2d_reg::Client>, Clip: ClipMode> Task
    for Fill<Buffer, Clip>
where
    Buffer::Format: Gr2dFormat,
{
    type Client = gr2d_reg::Client;
    type Buffers = Buffer;

    fn buffers(self) -> Self::Buffers {
        self.buffer
    }

    fn cmdseq(&mut self) -> impl CommandSequence<gr2d_reg::Client> {
        let width = self.buffer.width();
        let height = self.buffer.height();
        let stride = self.buffer.stride();
        (
            SetClass::<_, _, gr2d_reg::Client, _>::new(),
            Mask::<2, 0, gr2d_reg::Client, _>::new((TRIGGER0::raw(0x3a), CMDSEL::OUTPUT::MEMORY)),
            self.clip.cmdseq::<Buffer::Format>(),
            Mask::<2, 1, gr2d_reg::Client, _>::new((
                DSTBA::write(&mut self.buffer),
                DSTST::DSTS.val(stride as _),
            )),
            WriteNoIncr::new([SRCFGC::SRCFGC.val(Buffer::Format::color_to_gr2d_reg(self.color))]),
            WriteNoIncr::new([TILEMODE::zero()]),
            Mask::<2, 0, gr2d_reg::Client, _>::new((
                DSTSIZE::DSTWIDTH.val(width as _) + DSTSIZE::DSTHEIGHT.val(height as _),
                DSTPS::DSTX.val(0) + DSTPS::DSTY.val(0),
            )),
        )
    }
}
