use core::{fmt::Debug, marker::PhantomData, num::NonZeroUsize};

use arbitrary_int::{prelude::*, traits::UnsignedInteger};
use bytemuck::{AnyBitPattern, Pod};
use drm_fourcc::DrmFourcc;
use paste::paste;

use crate::color::Color;

mod sealed {
    pub trait SealedEncoding {}
    pub trait SealedLayout {}
    pub trait DefaultAlpha {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BitsPerPixel {
    BPP8 = 8,
    BPP16 = 16,
    BPP24 = 24,
    BPP32 = 32,
}
impl BitsPerPixel {
    const fn try_from_usize(v: usize) -> Self {
        match v {
            8 => BitsPerPixel::BPP8,
            16 => BitsPerPixel::BPP16,
            24 => BitsPerPixel::BPP24,
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

/// Defines pixel memory layout.
///
/// That is:
/// - defines pixel value size and alignment.
/// - assigns each channel plane number, byte, and bit-offset.
pub trait Layout: Copy + sealed::SealedLayout {
    /// Total number of planes used.
    ///
    /// Linear layouts have one plane.
    const N_PLANES: NonZeroUsize;

    /// Size of a pixel in memory.
    const BPP: BitsPerPixel;

    // TODO: implement missing consts/functions. Currently we have no means to
    // query actual layout so this trait is not usable.
}
pub trait RgbLayout: Layout {
    /// Red channel bit offset.
    const R_OFFSET: usize;

    /// Green channel bit offset.
    const G_OFFSET: usize;

    /// Blue channel bit offset.
    const B_OFFSET: usize;

    /// Plane number for red channel.
    const R_PLANE: usize;

    /// Plane number for green channel.
    const G_PLANE: usize;

    /// Plane number for blue channel.
    const B_PLANE: usize;
}
pub trait RgbaLayout: RgbLayout {
    /// Alpha channel bit offset.
    const A_OFFSET: usize;

    /// Plane number for alpha channel.
    const A_PLANE: usize;
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

#[derive(Clone, Copy)]
pub struct LinearRgb<
    const R_OFFSET: usize,
    const G_OFFSET: usize,
    const B_OFFSET: usize,
    const BPP: usize,
> {}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize, const BPP: usize>
    sealed::SealedLayout for LinearRgb<R_OFFSET, G_OFFSET, B_OFFSET, BPP>
{
}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize, const BPP: usize> Layout
    for LinearRgb<R_OFFSET, G_OFFSET, B_OFFSET, BPP>
{
    const N_PLANES: NonZeroUsize = NonZeroUsize::new(1).unwrap();
    const BPP: BitsPerPixel = BitsPerPixel::try_from_usize(BPP);
}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize, const BPP: usize>
    RgbLayout for LinearRgb<R_OFFSET, G_OFFSET, B_OFFSET, BPP>
{
    const R_OFFSET: usize = R_OFFSET;
    const G_OFFSET: usize = G_OFFSET;
    const B_OFFSET: usize = B_OFFSET;

    const R_PLANE: usize = 0;
    const G_PLANE: usize = 0;
    const B_PLANE: usize = 0;
}

#[derive(Clone, Copy)]
pub struct LinearRgba<
    const R_OFFSET: usize,
    const G_OFFSET: usize,
    const B_OFFSET: usize,
    const A_OFFSET: usize,
    const BPP: usize,
> {}
impl<
    const R_OFFSET: usize,
    const G_OFFSET: usize,
    const B_OFFSET: usize,
    const A_OFFSET: usize,
    const BPP: usize,
> sealed::SealedLayout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET, BPP>
{
}
impl<
    const R_OFFSET: usize,
    const G_OFFSET: usize,
    const B_OFFSET: usize,
    const A_OFFSET: usize,
    const BPP: usize,
> Layout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET, BPP>
{
    const N_PLANES: NonZeroUsize = NonZeroUsize::new(1).unwrap();
    const BPP: BitsPerPixel = BitsPerPixel::try_from_usize(BPP);
}
impl<
    const R_OFFSET: usize,
    const G_OFFSET: usize,
    const B_OFFSET: usize,
    const A_OFFSET: usize,
    const BPP: usize,
> RgbLayout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET, BPP>
{
    const R_OFFSET: usize = R_OFFSET;
    const G_OFFSET: usize = G_OFFSET;
    const B_OFFSET: usize = B_OFFSET;

    const R_PLANE: usize = 0;
    const G_PLANE: usize = 0;
    const B_PLANE: usize = 0;
}
impl<
    const R_OFFSET: usize,
    const G_OFFSET: usize,
    const B_OFFSET: usize,
    const A_OFFSET: usize,
    const BPP: usize,
> RgbaLayout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET, BPP>
{
    const A_OFFSET: usize = A_OFFSET;
    const A_PLANE: usize = 0;
}

pub trait Format: Debug + Copy {
    /// Pixel value encoding.
    type Encoding: Encoding;

    /// Channel in-memory layout.
    type Layout: Layout;

    /// Direct Rendering Manager Four Character Code for current format.
    const DRM_FCC: DrmFourcc;

    fn get_pixel(
        x: u32,
        y: u32,
        planes: &[(u32, &[u8])],
        bpp: BitsPerPixel,
    ) -> Option<Color<Self::Encoding>>;

    fn set_pixel(
        x: u32,
        y: u32,
        planes: &mut [(u32, &mut [u8])],
        bpp: BitsPerPixel,
        color: Color<Self::Encoding>,
    ) -> bool;

    fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32;
}

macro_rules! impl_format_linear {
    ($ty:ident, rgb = ($r_bits:literal, $g_bits:literal, $b_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                // DRM fourcc is MSB->LSB of a little-endian word: `R:G:B` means B is at byte 0.
                type Layout = LinearRgb<{ $b_bits + $g_bits }, $b_bits, 0, { $r_bits + $g_bits + $b_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgb_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgb_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, bgr = ($b_bits:literal, $g_bits:literal, $r_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                // DRM fourcc is MSB->LSB of a little-endian word: `B:G:R` means R is at byte 0.
                type Layout = LinearRgb::<0, $r_bits, { $r_bits + $g_bits }, { $r_bits + $g_bits + $b_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgb_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgb_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, xrgb = ($a_bits:literal, $r_bits:literal, $b_bits:literal, $g_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                // DRM fourcc is MSB->LSB of a little-endian word: `x:R:G:B` means B is at byte 0.
                type Layout = LinearRgb::<{ $b_bits + $g_bits }, $b_bits, 0, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgb_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgb_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, argb = ($a_bits:literal, $r_bits:literal, $b_bits:literal, $g_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                // DRM fourcc is MSB->LSB of a little-endian word: `A:R:G:B` means B is at byte 0.
                type Layout = LinearRgba::<{ $b_bits + $g_bits }, $b_bits, 0, { $b_bits + $g_bits + $r_bits }, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgba_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgba_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, xbgr = ($a_bits:literal, $b_bits:literal, $g_bits:literal, $r_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                // DRM fourcc is MSB->LSB of a little-endian word: `x:B:G:R` means R is at byte 0.
                type Layout = LinearRgb::<0, $r_bits, { $r_bits + $g_bits }, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgb_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgb_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, abgr = ($a_bits:literal, $b_bits:literal, $g_bits:literal, $r_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                // DRM fourcc is MSB->LSB of a little-endian word: `A:B:G:R` means R is at byte 0.
                type Layout = LinearRgba::<0, $r_bits, { $r_bits + $g_bits }, { $r_bits + $g_bits + $b_bits }, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgba_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgba_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, rgbx = ($r_bits:literal, $g_bits:literal, $b_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                // DRM fourcc is MSB->LSB of a little-endian word: `R:G:B:x` means x is at byte 0.
                type Layout = LinearRgb::<{ $a_bits + $b_bits + $g_bits }, { $a_bits + $b_bits }, $a_bits, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgb_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgb_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, rgba = ($r_bits:literal, $g_bits:literal, $b_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                // DRM fourcc is MSB->LSB of a little-endian word: `R:G:B:A` means A is at byte 0.
                type Layout = LinearRgba::<{ $a_bits + $b_bits + $g_bits }, { $a_bits + $b_bits }, $a_bits, 0, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgba_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgba_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, bgrx = ($b_bits:literal, $g_bits:literal, $r_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                // DRM fourcc is MSB->LSB of a little-endian word: `B:G:R:x` means x is at byte 0.
                type Layout = LinearRgb::<{ $a_bits + $r_bits + $g_bits }, { $a_bits + $r_bits }, $a_bits, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgb_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgb_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
    ($ty:ident, bgra = ($b_bits:literal, $g_bits:literal, $r_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                // DRM fourcc is MSB->LSB of a little-endian word: `B:G:R:A` means A is at byte 0.
                type Layout = LinearRgba::<$a_bits, { $a_bits + $r_bits }, { $a_bits + $r_bits + $g_bits }, 0, { $r_bits + $g_bits + $b_bits + $a_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }

                fn get_pixel(
                    x: u32,
                    y: u32,
                    planes: &[(u32, &[u8])],
                    bpp: BitsPerPixel,
                ) -> Option<Color<Self::Encoding>> { get_rgba_pixel::<Self>(x, y, planes, bpp) }

                fn set_pixel(
                    x: u32,
                    y: u32,
                    planes: &mut [(u32, &mut [u8])],
                    bpp: BitsPerPixel,
                    color: Color<Self::Encoding>,
                ) -> bool { set_rgba_pixel::<Self>(x, y, planes, bpp, color) }
            }
        }
    };
}

// TODO: deduplicate code by merging rgb and rgba functions into one.
fn rgb_to_gr2d_reg<F: Format>(stor: <F::Encoding as Encoding>::Storage) -> u32
where
    F::Encoding: Rgb,
    F::Layout: RgbLayout,
    <<F::Encoding as Rgb>::R as Integer>::UnderlyingType: Into<u32>,
    <<F::Encoding as Rgb>::G as Integer>::UnderlyingType: Into<u32>,
    <<F::Encoding as Rgb>::B as Integer>::UnderlyingType: Into<u32>,
{
    let mut v = u32::MAX;
    let m = <F::Encoding as Rgb>::R::MAX.value().into() << <F::Layout as RgbLayout>::R_OFFSET
        | <F::Encoding as Rgb>::G::MAX.value().into() << <F::Layout as RgbLayout>::G_OFFSET
        | <F::Encoding as Rgb>::B::MAX.value().into() << <F::Layout as RgbLayout>::B_OFFSET;
    v &= !m;
    v | <F::Encoding as Rgb>::r(&stor).value().into() << <F::Layout as RgbLayout>::R_OFFSET
        | <F::Encoding as Rgb>::g(&stor).value().into() << <F::Layout as RgbLayout>::G_OFFSET
        | <F::Encoding as Rgb>::b(&stor).value().into() << <F::Layout as RgbLayout>::B_OFFSET
}

fn rgba_to_gr2d_reg<F: Format>(stor: <F::Encoding as Encoding>::Storage) -> u32
where
    F::Encoding: RgbAlpha,
    F::Layout: RgbaLayout,
    <<F::Encoding as Rgb>::R as Integer>::UnderlyingType: Into<u32>,
    <<F::Encoding as Rgb>::G as Integer>::UnderlyingType: Into<u32>,
    <<F::Encoding as Rgb>::B as Integer>::UnderlyingType: Into<u32>,
    <<F::Encoding as RgbAlpha>::A as Integer>::UnderlyingType: Into<u32>,
{
    <F::Encoding as Rgb>::r(&stor).value().into() << <F::Layout as RgbLayout>::R_OFFSET
        | <F::Encoding as Rgb>::g(&stor).value().into() << <F::Layout as RgbLayout>::G_OFFSET
        | <F::Encoding as Rgb>::b(&stor).value().into() << <F::Layout as RgbLayout>::B_OFFSET
        | <F::Encoding as RgbAlpha>::a(&stor).value().into() << <F::Layout as RgbaLayout>::A_OFFSET
}

fn get_ch<T: UnsignedInteger<UnderlyingType: AnyBitPattern>>(
    buf: &[u8],
    stride: u32,
    x: u32,
    y: u32,
    bpp: BitsPerPixel,
    ch_off: usize,
) -> Option<T> {
    // TODO: accept ch_off as generic parameter when stabilized? Currently we need #![feature(generic_const_exprs)]

    // We'll get to this later
    debug_assert!(ch_off % 8 == 0);

    let pixel_off = y as usize * stride as usize + x as usize * (bpp.as_u8() as usize / 8);
    let off = pixel_off + ch_off / 8;
    let n_bytes = T::BITS.div_ceil(8);
    let v = buf.get(off..off + n_bytes)?;

    Some(T::new(*bytemuck::from_bytes(v)))
}

fn get_rgb_pixel<F: Format>(
    x: u32,
    y: u32,
    planes: &[(u32, &[u8])],
    bpp: BitsPerPixel,
) -> Option<Color<<F as Format>::Encoding>>
where
    F::Encoding: Rgb,
    F::Layout: RgbLayout,
{
    let &(r_plane_stride, r_plane) = planes.get(<F::Layout as RgbLayout>::R_PLANE)?;
    let &(g_plane_stride, g_plane) = planes.get(<F::Layout as RgbLayout>::G_PLANE)?;
    let &(b_plane_stride, b_plane) = planes.get(<F::Layout as RgbLayout>::B_PLANE)?;

    let r = get_ch::<<F::Encoding as Rgb>::R>(
        r_plane,
        r_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::R_OFFSET,
    )?;
    let g = get_ch::<<F::Encoding as Rgb>::G>(
        g_plane,
        g_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::G_OFFSET,
    )?;
    let b = get_ch::<<F::Encoding as Rgb>::B>(
        b_plane,
        b_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::B_OFFSET,
    )?;

    let mut c = <F::Encoding as Encoding>::BLACK;
    c.set_r(r);
    c.set_g(g);
    c.set_b(b);
    Some(c)
}

fn get_rgba_pixel<F: Format>(
    x: u32,
    y: u32,
    planes: &[(u32, &[u8])],
    bpp: BitsPerPixel,
) -> Option<Color<<F as Format>::Encoding>>
where
    F::Encoding: RgbAlpha,
    F::Layout: RgbaLayout,
{
    let &(r_plane_stride, r_plane) = planes.get(<F::Layout as RgbLayout>::R_PLANE)?;
    let &(g_plane_stride, g_plane) = planes.get(<F::Layout as RgbLayout>::G_PLANE)?;
    let &(b_plane_stride, b_plane) = planes.get(<F::Layout as RgbLayout>::B_PLANE)?;
    let &(a_plane_stride, a_plane) = planes.get(<F::Layout as RgbaLayout>::A_PLANE)?;

    let r = get_ch::<<F::Encoding as Rgb>::R>(
        r_plane,
        r_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::R_OFFSET,
    )?;
    let g = get_ch::<<F::Encoding as Rgb>::G>(
        g_plane,
        g_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::G_OFFSET,
    )?;
    let b = get_ch::<<F::Encoding as Rgb>::B>(
        b_plane,
        b_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::B_OFFSET,
    )?;
    let a = get_ch::<<F::Encoding as RgbAlpha>::A>(
        a_plane,
        a_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbaLayout>::A_OFFSET,
    )?;

    let mut c = <F::Encoding as Encoding>::BLACK;
    c.set_r(r);
    c.set_g(g);
    c.set_b(b);
    c.set_a(a);
    Some(c)
}

fn set_ch<T: UnsignedInteger<UnderlyingType: Pod>>(
    buf: &mut [u8],
    stride: u32,
    x: u32,
    y: u32,
    bpp: BitsPerPixel,
    ch_off: usize,
    val: T,
) -> bool {
    // TODO: accept ch_off as generic parameter when stabilized? Currently we need #![feature(generic_const_exprs)]

    // We'll get to this later
    debug_assert!(ch_off % 8 == 0);

    let pixel_off = y as usize * stride as usize + x as usize * (bpp.as_u8() as usize / 8);
    let off = pixel_off + ch_off / 8;
    let n_bytes = T::BITS.div_ceil(8);
    let Some(v) = buf.get_mut(off..off + n_bytes) else {
        return false;
    };
    v.copy_from_slice(bytemuck::bytes_of(&val.value()));

    true
}

fn set_rgb_pixel<F: Format>(
    x: u32,
    y: u32,
    planes: &mut [(u32, &mut [u8])],
    bpp: BitsPerPixel,
    color: Color<<F as Format>::Encoding>,
) -> bool
where
    F::Encoding: Rgb,
    F::Layout: RgbLayout,
{
    macro_rules! some_or_false {
        ($v:expr) => {
            match $v {
                Some(t) => t,
                None => return false,
            }
        };
    }

    // Check here that all channels are present before we do any changes to underlying data.
    //
    // I believe compiler should optimize further calls to planes.get_mut(), but I haven't checked it.
    // As we need to use get_mut() instead of get() to modify anything through dereference we can't hold
    // references to multiple planes at the same time.
    some_or_false!(planes.get(<F::Layout as RgbLayout>::R_PLANE));
    some_or_false!(planes.get(<F::Layout as RgbLayout>::G_PLANE));
    some_or_false!(planes.get(<F::Layout as RgbLayout>::B_PLANE));

    let (r_plane_stride, r_plane) =
        some_or_false!(planes.get_mut(<F::Layout as RgbLayout>::R_PLANE));
    set_ch::<<F::Encoding as Rgb>::R>(
        &mut **r_plane,
        *r_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::R_OFFSET,
        color.r(),
    );

    let (g_plane_stride, g_plane) =
        some_or_false!(planes.get_mut(<F::Layout as RgbLayout>::G_PLANE));
    set_ch::<<F::Encoding as Rgb>::G>(
        &mut **g_plane,
        *g_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::G_OFFSET,
        color.g(),
    );

    let (b_plane_stride, b_plane) =
        some_or_false!(planes.get_mut(<F::Layout as RgbLayout>::B_PLANE));
    set_ch::<<F::Encoding as Rgb>::B>(
        &mut **b_plane,
        *b_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::B_OFFSET,
        color.b(),
    );

    true
}

fn set_rgba_pixel<F: Format>(
    x: u32,
    y: u32,
    planes: &mut [(u32, &mut [u8])],
    bpp: BitsPerPixel,
    color: Color<<F as Format>::Encoding>,
) -> bool
where
    F::Encoding: RgbAlpha,
    F::Layout: RgbaLayout,
{
    macro_rules! some_or_false {
        ($v:expr) => {
            match $v {
                Some(t) => t,
                None => return false,
            }
        };
    }

    // Check here that all channels are present before we do any changes to underlying data.
    //
    // I believe compiler should optimize further calls to planes.get_mut(), but I haven't checked it.
    // As we need to use get_mut() instead of get() to modify anything through dereference we can't hold
    // references to multiple planes at the same time.
    some_or_false!(planes.get(<F::Layout as RgbLayout>::R_PLANE));
    some_or_false!(planes.get(<F::Layout as RgbLayout>::G_PLANE));
    some_or_false!(planes.get(<F::Layout as RgbLayout>::B_PLANE));
    some_or_false!(planes.get(<F::Layout as RgbaLayout>::A_PLANE));

    let (r_plane_stride, r_plane) =
        some_or_false!(planes.get_mut(<F::Layout as RgbLayout>::R_PLANE));
    set_ch::<<F::Encoding as Rgb>::R>(
        &mut **r_plane,
        *r_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::R_OFFSET,
        color.r(),
    );

    let (g_plane_stride, g_plane) =
        some_or_false!(planes.get_mut(<F::Layout as RgbLayout>::G_PLANE));
    set_ch::<<F::Encoding as Rgb>::G>(
        &mut **g_plane,
        *g_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::G_OFFSET,
        color.g(),
    );

    let (b_plane_stride, b_plane) =
        some_or_false!(planes.get_mut(<F::Layout as RgbLayout>::B_PLANE));
    set_ch::<<F::Encoding as Rgb>::B>(
        &mut **b_plane,
        *b_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbLayout>::B_OFFSET,
        color.b(),
    );
    let (a_plane_stride, a_plane) =
        some_or_false!(planes.get_mut(<F::Layout as RgbaLayout>::A_PLANE));
    set_ch::<<F::Encoding as RgbAlpha>::A>(
        &mut **a_plane,
        *a_plane_stride,
        x,
        y,
        bpp,
        <F::Layout as RgbaLayout>::A_OFFSET,
        color.a(),
    );

    true
}

impl_format_linear!(Abgr8888, abgr = (8, 8, 8, 8));
impl_format_linear!(Argb8888, argb = (8, 8, 8, 8));
impl_format_linear!(Bgr565, bgr = (5, 6, 5));
impl_format_linear!(Bgr888, bgr = (8, 8, 8));
impl_format_linear!(Bgra8888, bgra = (8, 8, 8, 8));
impl_format_linear!(Bgrx8888, bgrx = (8, 8, 8, 8));
impl_format_linear!(Rgb565, rgb = (5, 6, 5));
impl_format_linear!(Rgb888, rgb = (8, 8, 8));
impl_format_linear!(Rgba8888, rgba = (8, 8, 8, 8));
impl_format_linear!(Rgbx8888, rgbx = (8, 8, 8, 8));
impl_format_linear!(Xbgr8888, xbgr = (8, 8, 8, 8));
impl_format_linear!(Xrgb8888, xrgb = (8, 8, 8, 8));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    /// Store pure red into a single 32bpp pixel and return the 4 memory bytes.
    fn red_bytes<F: Format<Encoding = RgbGeneric<u8, u8, u8, u8>>>() -> [u8; 4] {
        let red = Color::<F::Encoding>::rgb(255, 0, 0);
        let mut buf = [0u8; 4];
        let mut planes: [(u32, &mut [u8]); 1] = [(4, &mut buf)];
        assert!(F::set_pixel(0, 0, &mut planes, BitsPerPixel::BPP32, red));
        buf
    }

    // DRM fourccs are MSB->LSB of a little-endian word, so the last-named channel
    // lands in byte 0. Alpha defaults to fully opaque (0xff).

    #[test]
    fn bgra8888_byte_order() {
        // BGRA8888 in memory is [A, R, G, B].
        assert_eq!(red_bytes::<Bgra8888>(), [0xff, 0xff, 0x00, 0x00]);
        let red = Color::<<Bgra8888 as Format>::Encoding>::rgb(255, 0, 0);
        assert_eq!(Bgra8888::color_to_gr2d_reg(red), 0x0000_ffff);
    }

    #[test]
    fn rgba8888_byte_order() {
        // RGBA8888 in memory is [A, B, G, R].
        assert_eq!(red_bytes::<Rgba8888>(), [0xff, 0x00, 0x00, 0xff]);
        let red = Color::<<Rgba8888 as Format>::Encoding>::rgb(255, 0, 0);
        assert_eq!(Rgba8888::color_to_gr2d_reg(red), 0xff00_00ff);
    }

    #[test]
    fn argb8888_byte_order() {
        // ARGB8888 in memory is [B, G, R, A].
        assert_eq!(red_bytes::<Argb8888>(), [0x00, 0x00, 0xff, 0xff]);
    }

    #[test]
    fn abgr8888_byte_order() {
        // ABGR8888 in memory is [R, G, B, A].
        assert_eq!(red_bytes::<Abgr8888>(), [0xff, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn bits_per_pixel() {
        fn bpp<F: Format>() -> u8 {
            <F::Layout as Layout>::BPP.as_u8()
        }
        assert_eq!(bpp::<Abgr8888>(), 32);
        assert_eq!(bpp::<Argb8888>(), 32);
        assert_eq!(bpp::<Bgr565>(), 16);
        assert_eq!(bpp::<Bgr888>(), 24);
        assert_eq!(bpp::<Bgra8888>(), 32);
        assert_eq!(bpp::<Bgrx8888>(), 32);
        assert_eq!(bpp::<Rgb565>(), 16);
        assert_eq!(bpp::<Rgb888>(), 24);
        assert_eq!(bpp::<Rgba8888>(), 32);
        assert_eq!(bpp::<Rgbx8888>(), 32);
        assert_eq!(bpp::<Xbgr8888>(), 32);
        assert_eq!(bpp::<Xrgb8888>(), 32);
    }
}
