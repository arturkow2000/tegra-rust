//! Pixel encodings: which channels a color has and how many bits each takes.
//!
//! Unlike [formats](crate::Format), encodings say nothing about memory layout, so
//! e.g. [`Rgba8`] is shared by all 8-bit RGBA formats regardless of channel order.

use core::marker::PhantomData;

use arbitrary_int::{
    prelude::{u5, u6},
    traits::UnsignedInteger,
};
use bytemuck::{AnyBitPattern, Pod};

use crate::color::Color;

mod sealed {
    pub trait SealedEncoding {}
    pub trait DefaultAlpha {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BitsPerPixel {
    BPP8 = 8,
    BPP16 = 16,
    BPP32 = 32,
}
impl BitsPerPixel {
    const fn try_from_usize(v: usize) -> Self {
        match v {
            8 => BitsPerPixel::BPP8,
            16 => BitsPerPixel::BPP16,
            32 => BitsPerPixel::BPP32,
            _ => panic!("unsupported BPP value"),
        }
    }
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Pixel encoding, defines what channels form color of a pixel and bitsize of each channel value.
pub trait Encoding: Copy + sealed::SealedEncoding {
    type Storage: Copy + Eq;
    const BPP: BitsPerPixel;

    const BLACK: Color<Self>;

    fn enumerate_channels(stor: &Self::Storage) -> impl Iterator<Item = (&'static str, u32)>;
}

/// An RGB (Red Green Blue) encoding.
pub trait Rgb: Encoding {
    /// Underlying type used for red channel value.
    type R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + 'static;

    /// Underlying type used for green channel value.
    type G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + 'static;

    /// Underlying type used for blue channel value.
    type B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + 'static;

    /// Get red channel value.
    fn r(stor: &Self::Storage) -> Self::R;

    /// Get green channel value.
    fn g(stor: &Self::Storage) -> Self::G;

    /// Get blue channel value.
    fn b(stor: &Self::Storage) -> Self::B;

    /// Set red channel value.
    fn set_r(stor: &mut Self::Storage, r: Self::R);

    /// Set green channel value.
    fn set_g(stor: &mut Self::Storage, g: Self::G);

    /// Set blue channel value.
    fn set_b(stor: &mut Self::Storage, b: Self::B);
}

/// An RGB encoding with additional Alpha channel.
///
/// This trait is implemented for encodings that have a non-zero alpha channel size.
pub trait RgbAlpha: Rgb {
    type A: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + 'static;

    fn a(stor: &Self::Storage) -> Self::A;
    fn set_a(stor: &mut Self::Storage, a: Self::A);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Red color
    R,

    /// Green color
    G,

    /// Blue color
    B,

    /// Alpha channel
    A,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NoAlpha;
pub trait DefaultAlpha: sealed::DefaultAlpha {
    const ALPHA_BITS: u8;
    const DEFAULT_ALPHA: Self;
}
impl sealed::DefaultAlpha for NoAlpha {}
impl DefaultAlpha for NoAlpha {
    const ALPHA_BITS: u8 = 0;
    const DEFAULT_ALPHA: Self = NoAlpha;
}
impl<T: UnsignedInteger> sealed::DefaultAlpha for T {}
impl<T: UnsignedInteger> DefaultAlpha for T {
    const ALPHA_BITS: u8 = T::BITS as u8;
    const DEFAULT_ALPHA: Self = Self::MAX;
}

#[derive(Clone, Copy)]
pub struct RgbGeneric<R, G, B, A>(PhantomData<(R, G, B, A)>);
impl<
    R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    A: DefaultAlpha,
> RgbGeneric<R, G, B, A>
{
}
impl<
    R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
> sealed::SealedEncoding for RgbGeneric<R, G, B, NoAlpha>
{
}
impl<
    R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
> Encoding for RgbGeneric<R, G, B, NoAlpha>
{
    type Storage = (R, G, B, NoAlpha);
    const BPP: BitsPerPixel = BitsPerPixel::try_from_usize(R::BITS + G::BITS + B::BITS);

    const BLACK: Color<Self> = Color((R::MIN, G::MIN, B::MIN, NoAlpha));

    fn enumerate_channels(stor: &Self::Storage) -> impl Iterator<Item = (&'static str, u32)> {
        [
            ("r", Self::r(stor).into()),
            ("g", Self::g(stor).into()),
            ("b", Self::b(stor).into()),
        ]
        .into_iter()
    }

    /*fn to_gr2d_reg(v: Color<Self>) -> u32 {
        // TODO: implement
        0xffffffff
    }*/
}
impl<
    R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    A: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
> sealed::SealedEncoding for RgbGeneric<R, G, B, A>
{
}
impl<
    R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    A: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
> Encoding for RgbGeneric<R, G, B, A>
{
    type Storage = (R, G, B, A);
    const BPP: BitsPerPixel = BitsPerPixel::try_from_usize(R::BITS + G::BITS + B::BITS + A::BITS);

    const BLACK: Color<Self> = Color((R::MIN, G::MIN, B::MIN, A::DEFAULT_ALPHA));

    fn enumerate_channels(stor: &Self::Storage) -> impl Iterator<Item = (&'static str, u32)> {
        [
            ("r", Self::r(stor).into()),
            ("g", Self::g(stor).into()),
            ("b", Self::b(stor).into()),
            ("a", Self::a(stor).into()),
        ]
        .into_iter()
    }
}
impl<
    R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    A: DefaultAlpha,
> Rgb for RgbGeneric<R, G, B, A>
where
    Self: Encoding<Storage = (R, G, B, A)>,
{
    type R = R;
    type G = G;
    type B = B;

    #[inline]
    fn r(stor: &Self::Storage) -> Self::R {
        stor.0
    }

    #[inline]
    fn g(stor: &Self::Storage) -> Self::G {
        stor.1
    }

    #[inline]
    fn b(stor: &Self::Storage) -> Self::B {
        stor.2
    }

    #[inline]
    fn set_r(stor: &mut Self::Storage, r: Self::R) {
        stor.0 = r;
    }

    #[inline]
    fn set_g(stor: &mut Self::Storage, g: Self::G) {
        stor.1 = g;
    }

    #[inline]
    fn set_b(stor: &mut Self::Storage, b: Self::B) {
        stor.2 = b;
    }
}
impl<
    R: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    G: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    B: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
    A: UnsignedInteger<UnderlyingType: Pod + AnyBitPattern> + Into<u32> + 'static,
> RgbAlpha for RgbGeneric<R, G, B, A>
{
    type A = A;

    fn a(stor: &Self::Storage) -> Self::A {
        stor.3
    }

    fn set_a(stor: &mut Self::Storage, a: Self::A) {
        stor.3 = a;
    }
}

/// 8-bit RGB without alpha.
pub type Rgb8 = RgbGeneric<u8, u8, u8, NoAlpha>;

/// 8-bit RGB with 8-bit alpha.
pub type Rgba8 = RgbGeneric<u8, u8, u8, u8>;

/// RGB with 5-bit red, 6-bit green and 5-bit blue, without alpha.
pub type Rgb565 = RgbGeneric<u5, u6, u5, NoAlpha>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Format, color::Color};

    /// Aliases must name the same encodings formats use.
    #[test]
    fn aliases_match_formats() {
        let _: Color<Rgb8> = <crate::Rgb888 as Format>::Encoding::BLACK;
        let _: Color<Rgb8> = <crate::Xrgb8888 as Format>::Encoding::BLACK;
        let _: Color<Rgba8> = <crate::Argb8888 as Format>::Encoding::BLACK;
        let _: Color<Rgb565> = <crate::Rgb565 as Format>::Encoding::BLACK;
    }
}
