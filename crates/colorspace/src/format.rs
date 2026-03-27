use core::{fmt::Debug, marker::PhantomData, num::NonZeroUsize};

use arbitrary_int::{prelude::*, traits::UnsignedInteger};
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
pub trait Encoding: sealed::SealedEncoding {
    type Storage;
    const BPP: BitsPerPixel;

    const BLACK: Color<Self>;
}

/// An RGB (Red Green Blue) encoding.
pub trait Rgb: Encoding {
    /// Underlying type used for red channel value.
    type R: UnsignedInteger;

    /// Underlying type used for green channel value.
    type G: UnsignedInteger;

    /// Underlying type used for blue channel value.
    type B: UnsignedInteger;

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
    type A: UnsignedInteger;

    fn a(stor: &Self::Storage) -> Self::A;
    fn set_a(stor: &mut Self::Storage, a: Self::A);
}

/// Defines pixel memory layout.
///
/// That is:
/// - defines pixel value size and alignment.
/// - assigns each channel plane number, byte, and bit-offset.
pub trait Layout: sealed::SealedLayout {
    /// Total number of planes used.
    ///
    /// Linear layouts have one plane.
    const N_PLANES: NonZeroUsize;

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

#[derive(Clone, Copy)]
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
    R: UnsignedInteger + Into<u32>,
    G: UnsignedInteger + Into<u32>,
    B: UnsignedInteger + Into<u32>,
    A: DefaultAlpha,
> RgbGeneric<R, G, B, A>
{
}
impl<R: UnsignedInteger + Into<u32>, G: UnsignedInteger + Into<u32>, B: UnsignedInteger + Into<u32>>
    sealed::SealedEncoding for RgbGeneric<R, G, B, NoAlpha>
{
}
impl<R: UnsignedInteger + Into<u32>, G: UnsignedInteger + Into<u32>, B: UnsignedInteger + Into<u32>>
    Encoding for RgbGeneric<R, G, B, NoAlpha>
{
    type Storage = (R, G, B, NoAlpha);
    const BPP: BitsPerPixel = BitsPerPixel::try_from_usize(R::BITS + G::BITS + B::BITS);

    const BLACK: Color<Self> = Color((R::MIN, G::MIN, B::MIN, NoAlpha));

    /*fn to_gr2d_reg(v: Color<Self>) -> u32 {
        // TODO: implement
        0xffffffff
    }*/
}
impl<
    R: UnsignedInteger + Into<u32>,
    G: UnsignedInteger + Into<u32>,
    B: UnsignedInteger + Into<u32>,
    A: UnsignedInteger + Into<u32>,
> sealed::SealedEncoding for RgbGeneric<R, G, B, A>
{
}
impl<
    R: UnsignedInteger + Into<u32>,
    G: UnsignedInteger + Into<u32>,
    B: UnsignedInteger + Into<u32>,
    A: UnsignedInteger + Into<u32>,
> Encoding for RgbGeneric<R, G, B, A>
{
    type Storage = (R, G, B, A);
    const BPP: BitsPerPixel = BitsPerPixel::try_from_usize(R::BITS + G::BITS + B::BITS + A::BITS);

    const BLACK: Color<Self> = Color((R::MIN, G::MIN, B::MIN, A::DEFAULT_ALPHA));
}
impl<
    R: UnsignedInteger + Into<u32>,
    G: UnsignedInteger + Into<u32>,
    B: UnsignedInteger + Into<u32>,
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
    R: UnsignedInteger + Into<u32>,
    G: UnsignedInteger + Into<u32>,
    B: UnsignedInteger + Into<u32>,
    A: UnsignedInteger + Into<u32>,
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

pub struct LinearRgb<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize> {}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize> sealed::SealedLayout
    for LinearRgb<R_OFFSET, G_OFFSET, B_OFFSET>
{
}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize> Layout
    for LinearRgb<R_OFFSET, G_OFFSET, B_OFFSET>
{
    const N_PLANES: NonZeroUsize = NonZeroUsize::new(1).unwrap();
}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize> RgbLayout
    for LinearRgb<R_OFFSET, G_OFFSET, B_OFFSET>
{
    const R_OFFSET: usize = R_OFFSET;
    const G_OFFSET: usize = G_OFFSET;
    const B_OFFSET: usize = B_OFFSET;

    const R_PLANE: usize = 0;
    const G_PLANE: usize = 0;
    const B_PLANE: usize = 0;
}

pub struct LinearRgba<
    const R_OFFSET: usize,
    const G_OFFSET: usize,
    const B_OFFSET: usize,
    const A_OFFSET: usize,
> {}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize, const A_OFFSET: usize>
    sealed::SealedLayout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET>
{
}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize, const A_OFFSET: usize>
    Layout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET>
{
    const N_PLANES: NonZeroUsize = NonZeroUsize::new(1).unwrap();
}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize, const A_OFFSET: usize>
    RgbLayout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET>
{
    const R_OFFSET: usize = R_OFFSET;
    const G_OFFSET: usize = G_OFFSET;
    const B_OFFSET: usize = B_OFFSET;

    const R_PLANE: usize = 0;
    const G_PLANE: usize = 0;
    const B_PLANE: usize = 0;
}
impl<const R_OFFSET: usize, const G_OFFSET: usize, const B_OFFSET: usize, const A_OFFSET: usize>
    RgbaLayout for LinearRgba<R_OFFSET, G_OFFSET, B_OFFSET, A_OFFSET>
{
    const A_OFFSET: usize = A_OFFSET;
    const A_PLANE: usize = 0;
}

pub trait Format: Debug {
    /// Pixel value encoding.
    type Encoding: Encoding;

    /// Channel in-memory layout.
    type Layout: Layout;

    /// Direct Rendering Manager Four Character Code for current format.
    const DRM_FCC: DrmFourcc;

    fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32;
}

macro_rules! impl_format_linear {
    ($ty:ident, rgb = ($r_bits:literal, $g_bits:literal, $b_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                type Layout = LinearRgb<0, $r_bits, { $r_bits + $g_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, bgr = ($b_bits:literal, $g_bits:literal, $r_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                type Layout = LinearRgb::<{ $b_bits + $g_bits }, $b_bits, 0>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, xrgb = ($a_bits:literal, $r_bits:literal, $b_bits:literal, $g_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                type Layout = LinearRgb::<$a_bits, { $a_bits + $r_bits }, { $a_bits + $r_bits + $g_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, argb = ($a_bits:literal, $r_bits:literal, $b_bits:literal, $g_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                type Layout = LinearRgba::<0, $a_bits, { $a_bits + $r_bits }, { $a_bits + $r_bits + $g_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, xbgr = ($a_bits:literal, $b_bits:literal, $g_bits:literal, $r_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                type Layout = LinearRgb::<{ $a_bits + $b_bits + $g_bits }, { $a_bits + $b_bits }, $a_bits>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, abgr = ($a_bits:literal, $b_bits:literal, $g_bits:literal, $r_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                type Layout = LinearRgba::<{ $a_bits + $b_bits + $g_bits }, { $a_bits + $b_bits }, $a_bits, 0>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, rgbx = ($r_bits:literal, $g_bits:literal, $b_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                type Layout = LinearRgb::<0, $r_bits, { $r_bits + $g_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, rgba = ($r_bits:literal, $g_bits:literal, $b_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                type Layout = LinearRgba::<0, $r_bits, { $r_bits + $g_bits }, { $r_bits + $g_bits + $b_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, bgrx = ($b_bits:literal, $g_bits:literal, $r_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], NoAlpha>;
                type Layout = LinearRgb::<{ $b_bits + $g_bits }, $b_bits, 0>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgb_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
    ($ty:ident, bgra = ($b_bits:literal, $g_bits:literal, $r_bits:literal, $a_bits:literal)) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            pub struct $ty {}
            impl Format for $ty {
                type Encoding = RgbGeneric::<[<u $r_bits>], [<u $g_bits>], [<u $b_bits>], [<u $a_bits>]>;
                type Layout = LinearRgba::<{ $b_bits + $g_bits }, $b_bits, 0, { $b_bits + $g_bits + $r_bits }>;
                const DRM_FCC: DrmFourcc = DrmFourcc::$ty;

                fn color_to_gr2d_reg(c: Color<Self::Encoding>) -> u32 {
                    rgba_to_gr2d_reg::<Self>(c.0)
                }
            }
        }
    };
}

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
