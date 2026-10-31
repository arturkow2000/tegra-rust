use colorspace::{
    BitsPerPixel, Encoding, Format, NoAlpha, Rgb, RgbAlpha, RgbGeneric, color::Color,
};
use embedded_graphics::primitives::Rectangle;
use gr2d_reg::{
    ALPHABLEND, CLIPLEFTTOP, CLIPRIGHTBOT, CMDSEL, CTRL1, CTRLMAIN, DSTBA, DSTPS, DSTSIZE, DSTST,
    ROPFADE, SRCFGC, TILEMODE, TRIGGER0,
};
use host1x_buffer::{PixelBuffer, WritableBuffer};
use host1x_core::{
    cmd::{Mask, SetClass, WriteNoIncr},
    cmdseq::CommandSequence,
    task::Task,
};
use tock_registers::fields::FieldValue;

use crate::{format::Gr2dFormat, util::bpp_to_dstcd};

mod p {
    use colorspace::{Encoding, Format, color::Color};
    use host1x_core::cmdseq::CommandSequence;

    use crate::format::Gr2dFormat;

    pub trait ClipMode {
        fn cmdseq<F: Format + Gr2dFormat>(
            self,
            alpha: super::Alpha,
        ) -> impl CommandSequence<gr2d_reg::Client>;
    }

    pub trait FillColor<E: Encoding> {
        /// Color in the destination encoding and the alpha to blend it with.
        fn split(self) -> (Color<E>, super::Alpha);
    }
}

pub trait ClipMode: p::ClipMode {}

/// A color that can be filled into a buffer with encoding `E`.
///
/// Implemented for colors with the same RGB channels as `E`, with or without
/// 8-bit alpha, whether or not `E` has alpha:
/// - colors without alpha ([`NoAlpha`]) replace the pixels,
/// - colors with alpha replace the pixels when fully opaque and are alpha
///   blended onto them otherwise.
pub trait FillColor<E: Encoding>: p::FillColor<E> {}

impl<R, G, B, D: Rgb<R = R, G = G, B = B>> p::FillColor<D> for Color<RgbGeneric<R, G, B, NoAlpha>>
where
    RgbGeneric<R, G, B, NoAlpha>: Rgb<R = R, G = G, B = B>,
{
    #[inline]
    fn split(self) -> (Color<D>, Alpha) {
        (Color::rgb(self.r(), self.g(), self.b()), Alpha::OPAQUE)
    }
}
impl<R, G, B, D: Rgb<R = R, G = G, B = B>> FillColor<D> for Color<RgbGeneric<R, G, B, NoAlpha>> where
    RgbGeneric<R, G, B, NoAlpha>: Rgb<R = R, G = G, B = B>
{
}

impl<R, G, B, D: Rgb<R = R, G = G, B = B>> p::FillColor<D> for Color<RgbGeneric<R, G, B, u8>>
where
    RgbGeneric<R, G, B, u8>: RgbAlpha<R = R, G = G, B = B, A = u8>,
{
    #[inline]
    fn split(self) -> (Color<D>, Alpha) {
        (Color::rgb(self.r(), self.g(), self.b()), Alpha(self.a()))
    }
}
impl<R, G, B, D: Rgb<R = R, G = G, B = B>> FillColor<D> for Color<RgbGeneric<R, G, B, u8>> where
    RgbGeneric<R, G, B, u8>: RgbAlpha<R = R, G = G, B = B, A = u8>
{
}

/// Fixed alpha a fill is blended with. Fully opaque alpha disables blending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alpha(u8);
impl Alpha {
    const OPAQUE: Self = Self(0xff);

    /// CTRLMAIN value selecting either a plain fill or a blend.
    ///
    /// Both use the same registers, so the size of the command stream doesn't
    /// depend on the alpha value.
    fn ctrlmain<F: Format>(self, turbofill: bool) -> FieldValue<u32, CTRLMAIN::Register> {
        let bpp = <F as Format>::Encoding::BPP;
        let base = CTRLMAIN::SRCSLD::ENABLE
            + bpp_to_dstcd(bpp)
            + CTRLMAIN::SRCCD.val(probe::SRCCD.load(core::sync::atomic::Ordering::Relaxed))
            + CTRLMAIN::CMDT::BITBLT;
        if self == Self::OPAQUE {
            base + CTRLMAIN::ALPEN::DISABLE
                + if turbofill {
                    CTRLMAIN::TURBOFILL::ENABLE
                } else {
                    CTRLMAIN::TURBOFILL::DISABLE
                }
        } else {
            // Turbofill bypasses the blend unit.
            base + CTRLMAIN::ALPEN::ENABLE + CTRLMAIN::TURBOFILL::DISABLE
        }
    }

    fn alphablend(self) -> FieldValue<u32, ALPHABLEND::Register> {
        match probe::ALPHABLEND.load(core::sync::atomic::Ordering::Relaxed) {
            u32::MAX => ALPHABLEND::ALPHA.val(self.0 as _) + ALPHABLEND::ALPHAINV.val(!self.0 as _),
            v => ALPHABLEND::ALPHA.val(v & 0xff) + ALPHABLEND::ALPHAINV.val((v >> 8) & 0xff),
        }
    }
}

pub struct Clip(pub Rectangle);
impl p::ClipMode for Clip {
    fn cmdseq<F: Format + Gr2dFormat>(
        self,
        alpha: Alpha,
    ) -> impl host1x_core::cmdseq::CommandSequence<gr2d_reg::Client> {
        (Mask::<7, 0, gr2d_reg::Client, _>::new((
            F::SIFMT + F::DIFMT,
            CTRL1::CLIPC::DRAW_INSIDE + CTRL1::ALPTYPE.val(probe::ALPTYPE.load(core::sync::atomic::Ordering::Relaxed)) + CTRL1::ALPSRCORDST.val(probe::ALPSRCORDST.load(core::sync::atomic::Ordering::Relaxed)) + { let s = if alpha == Alpha::OPAQUE { 0 } else { probe::SWAP.load(core::sync::atomic::Ordering::Relaxed) }; CTRL1::BEBSWAP.val(s & 1) + CTRL1::BEWSWAP.val(s >> 1 & 1) },
            // Can't use turbofill with clipping
            alpha.ctrlmain::<F>(false),
            ROPFADE::ROP::COPY,
            alpha.alphablend(),
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
        self,
        alpha: Alpha,
    ) -> impl host1x_core::cmdseq::CommandSequence<gr2d_reg::Client> {
        (Mask::<5, 0, gr2d_reg::Client, _>::new((
            F::SIFMT + F::DIFMT,
            CTRL1::CLIPC::DISABLE + CTRL1::ALPTYPE.val(probe::ALPTYPE.load(core::sync::atomic::Ordering::Relaxed)) + CTRL1::ALPSRCORDST.val(probe::ALPSRCORDST.load(core::sync::atomic::Ordering::Relaxed)),
            alpha.ctrlmain::<F>(true),
            ROPFADE::ROP::COPY,
            alpha.alphablend(),
        )),)
    }
}
impl ClipMode for NoClip {}

pub struct Fill<Buffer: PixelBuffer, Clip> {
    buffer: Buffer,
    clip: Clip,
    color: Color<<Buffer::Format as Format>::Encoding>,
    alpha: Alpha,
}
impl<Buffer: PixelBuffer, Clip: ClipMode> Fill<Buffer, Clip> {
    /// Fill with `color`, blending it if it has non-opaque alpha.
    pub(crate) fn new(
        buffer: Buffer,
        clip: Clip,
        color: impl FillColor<<Buffer::Format as Format>::Encoding>,
    ) -> Self {
        let (color, alpha) = p::FillColor::split(color);
        Self {
            buffer,
            clip,
            color,
            alpha,
        }
    }

    /// Fill with `color` as is, including its alpha channel, without blending.
    pub(crate) fn new_raw(
        buffer: Buffer,
        clip: Clip,
        color: Color<<Buffer::Format as Format>::Encoding>,
    ) -> Self {
        Self {
            buffer,
            clip,
            color,
            alpha: Alpha::OPAQUE,
        }
    }
}
// Safety: the only relocation targets `buffer`, which is branded with 'id.
unsafe impl<'id, Buffer: PixelBuffer + WritableBuffer<'id, gr2d_reg::Client>, Clip: ClipMode>
    Task<'id> for Fill<Buffer, Clip>
where
    Buffer::Format: Gr2dFormat,
{
    type Client = gr2d_reg::Client;
    type Output = Buffer;

    fn split(self) -> (impl CommandSequence<gr2d_reg::Client>, Self::Output) {
        let Self {
            mut buffer,
            clip,
            color,
            alpha,
        } = self;
        let width = buffer.width();
        let height = buffer.height();
        let stride = buffer.stride();
        let seq = (
            SetClass::<_, _, gr2d_reg::Client, _>::new(),
            Mask::<2, 0, gr2d_reg::Client, _>::new((TRIGGER0::raw(0x3a), CMDSEL::OUTPUT::MEMORY)),
            clip.cmdseq::<Buffer::Format>(alpha),
            Mask::<2, 1, gr2d_reg::Client, _>::new((
                DSTBA::write(&mut buffer),
                DSTST::DSTS.val(stride as _),
            )),
            WriteNoIncr::new([SRCFGC::SRCFGC.val({ let c = Buffer::Format::color_to_gr2d_reg(color); let a = alpha.0 as u32; match probe::SRCFGC_MODE.load(core::sync::atomic::Ordering::Relaxed) { 1 => c & 0x00ff_ffff | a << 24, 2 => c & 0xffff_ff00 | a, 3 => (c & 0xffff_ff00 | a).swap_bytes(), _ => c } })]),
            WriteNoIncr::new([TILEMODE::zero()]),
            Mask::<2, 0, gr2d_reg::Client, _>::new((
                DSTSIZE::DSTWIDTH.val(width as _) + DSTSIZE::DSTHEIGHT.val(height as _),
                DSTPS::DSTX.val(0) + DSTPS::DSTY.val(0),
            )),
        );
        (seq, buffer)
    }
}

// TEMPORARY probe knobs, remove before commit.
pub mod probe {
    use core::sync::atomic::{AtomicBool, AtomicU32};
    pub static ALPTYPE: AtomicU32 = AtomicU32::new(0);
    pub static ALPSRCORDST: AtomicU32 = AtomicU32::new(0);
    pub static SRCCD: AtomicU32 = AtomicU32::new(0);
    // 0: as is, 1: alpha in [31:24], 2: alpha in [7:0], 3: swap_bytes of 2
    pub static SRCFGC_MODE: AtomicU32 = AtomicU32::new(0);
    // bit 0: BEBSWAP, bit 1: BEWSWAP, only applied when blending
    pub static SWAP: AtomicU32 = AtomicU32::new(0);
    pub static ALPHABLEND: AtomicU32 = AtomicU32::new(u32::MAX);
}
