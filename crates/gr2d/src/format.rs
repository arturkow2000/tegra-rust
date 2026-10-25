use gr2d_reg::SBFMT0::{self, DIFMT, SIFMT};
use tock_registers::fields::FieldValue;

/// A pixel format GR2D can render into.
///
/// GR2D names formats starting from the least significant bits (TRM 29.2.1), so
/// its B8G8R8A8 is DRM ARGB8888 and its R8G8B8A8 is DRM ABGR8888.
pub trait Gr2dFormat {
    const SIFMT: FieldValue<u32, SBFMT0::Register>;
    const DIFMT: FieldValue<u32, SBFMT0::Register>;
}

/// A [`Gr2dFormat`] GR2D can alpha blend.
///
/// The blend engine takes alpha from bits 31:24 of 32bpp pixels and can't swap
/// components, so only formats with a real alpha channel there qualify.
pub trait Gr2dBlendFormat: Gr2dFormat {}

impl Gr2dFormat for colorspace::Argb8888 {
    const SIFMT: FieldValue<u32, SBFMT0::Register> = SIFMT::B8G8R8A8;
    const DIFMT: FieldValue<u32, SBFMT0::Register> = DIFMT::B8G8R8A8;
}
impl Gr2dBlendFormat for colorspace::Argb8888 {}

impl Gr2dFormat for colorspace::Abgr8888 {
    const SIFMT: FieldValue<u32, SBFMT0::Register> = SIFMT::R8G8B8A8;
    const DIFMT: FieldValue<u32, SBFMT0::Register> = DIFMT::R8G8B8A8;
}
impl Gr2dBlendFormat for colorspace::Abgr8888 {}

// GR2D has no format with alpha in the lowest byte. Unless alpha blending is
// enabled GR2D only cares about color depth, so these still work for fills, but
// they must never implement Gr2dBlendFormat.
impl Gr2dFormat for colorspace::Rgba8888 {
    const SIFMT: FieldValue<u32, SBFMT0::Register> = SIFMT::R8G8B8A8;
    const DIFMT: FieldValue<u32, SBFMT0::Register> = DIFMT::R8G8B8A8;
}

impl Gr2dFormat for colorspace::Bgra8888 {
    const SIFMT: FieldValue<u32, SBFMT0::Register> = SIFMT::B8G8R8A8;
    const DIFMT: FieldValue<u32, SBFMT0::Register> = DIFMT::B8G8R8A8;
}
