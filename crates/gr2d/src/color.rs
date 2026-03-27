//! Module contains definitions for color formats supported by GR2D.
// TODO: maybe use https://crates.io/crates/rgb?

use core::{fmt, marker::PhantomData};

use arbitrary_int::prelude::*;
use drm_fourcc_ext::fourcc::Colorspace;
use paste::paste;

mod sealed {
    pub trait Sealed {}
}

// Hack to avoid using unstable const trait impls.
macro_rules! get_value {
    (8, $v:expr) => {
        $v as u8
    };
    ($n_bits:expr, $v:expr) => {
        $v.value()
    };
}
macro_rules! new_value {
    (8, $v:expr) => {
        $v as u8
    };
    ($n_bits:expr, $v:expr) => {
        paste! { [<u $n_bits>]::new($v) }
    };
}
macro_rules! zero {
    (8) => {
        new_value!(8, 0)
    };
    ($n_bits:expr) => {
        new_value!($n_bits, 0)
    };
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
#[repr(transparent)]
pub struct Color<F: Colorspace> {
    _format: PhantomData<F>,
}

macro_rules! impl_rgb_color {
    (
        $type:ident,
        $storage_type:ty,
        ($r_bits:expr, $g_bits:expr, $b_bits:expr),
        ($r_pos:expr, $g_pos:expr, $b_pos:expr),
    ) => {
        paste! {
            //#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
            //pub struct $type($storage_type);
            impl Color<drm_fourcc_ext::fourcc::$type> {
                const R_MASK: $storage_type = ($type::MAX_R as $storage_type) << $r_pos;
                const G_MASK: $storage_type = ($type::MAX_G as $storage_type) << $g_pos;
                const B_MASK: $storage_type = ($type::MAX_B as $storage_type) << $b_pos;
                const RGB_MASK: $storage_type = Self::R_MASK | Self::B_MASK | Self::G_MASK;

                const _MAX_R: [<u $r_bits>] = [<u $r_bits>]::MAX;
                const _MAX_G: [<u $g_bits>] = [<u $g_bits>]::MAX;
                const _MAX_B: [<u $b_bits>] = [<u $b_bits>]::MAX;

                pub const fn new(
                    r: [<u $r_bits>],
                    g: [<u $g_bits>],
                    b: [<u $b_bits>],
                ) -> Self {
                    // By default set 100% alpha (fully opaque).
                    let alpha = $storage_type::MAX & !Self::RGB_MASK;

                    Self(
                        alpha
                        | (get_value!($r_bits, r) as $storage_type) << $r_pos
                        | (get_value!($g_bits, g) as $storage_type) << $g_pos
                        | (get_value!($b_bits, b) as $storage_type) << $b_pos
                    )
                }
            }
            impl sealed::Sealed for $type {}
            impl PixelColor for $type {}
            impl RgbColor for $type {
                const MAX_R: u8 = get_value!($r_bits, [<u $r_bits>]::MAX);
                const MAX_G: u8 = get_value!($g_bits, [<u $g_bits>]::MAX);
                const MAX_B: u8 = get_value!($b_bits, [<u $b_bits>]::MAX);

                const BLACK: Self = Self::new(zero!($r_bits), zero!($g_bits), zero!($b_bits));
                const RED: Self = Self::new(Self::_MAX_R, zero!($g_bits), zero!($b_bits));
                const GREEN: Self = Self::new(zero!($r_bits), Self::_MAX_G, zero!($b_bits));
                const BLUE: Self = Self::new(zero!($r_bits), zero!($g_bits), Self::_MAX_B);
                const YELLOW: Self = Self::new(Self::_MAX_R, Self::_MAX_G, zero!($b_bits));
                const MAGENTA: Self = Self::new(Self::_MAX_R, zero!($g_bits), Self::_MAX_B);
                const CYAN: Self = Self::new(zero!($r_bits), Self::_MAX_G, Self::_MAX_B);
                const WHITE: Self = Self::new(Self::_MAX_R, Self::_MAX_G, Self::_MAX_B);

                fn r(&self) -> u8 {
                    ((self.0 & Self::R_MASK) >> $r_pos) as u8
                }
                fn g(&self) -> u8 {
                    ((self.0 & Self::G_MASK) >> $g_pos) as u8
                }
                fn b(&self) -> u8 {
                    ((self.0 & Self::B_MASK) >> $b_pos) as u8
                }
            }
        }
    };
}

macro_rules! impl_debug_rgb {
    ($type:ident) => {
        impl fmt::Debug for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let r = self.r();
                let g = self.g();
                let b = self.b();

                f.debug_struct(stringify!($type))
                    .field("r", &r)
                    .field("g", &g)
                    .field("b", &b)
                    .finish()
            }
        }
    };
}

macro_rules! impl_debug_rgba {
    ($type:ident) => {
        impl fmt::Debug for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let r = self.r();
                let g = self.g();
                let b = self.b();
                let a = self.a();

                f.debug_struct(stringify!($type))
                    .field("r", &r)
                    .field("g", &g)
                    .field("b", &b)
                    .field("a", &a)
                    .finish()
            }
        }
    };
}

macro_rules! impl_rgb_alpha_channel {
    (
        $type:ident,
        $storage_type:ty,
        ($r_bits:expr, $g_bits:expr, $b_bits:expr, $a_bits:expr),
        ($r_pos:expr, $g_pos:expr, $b_pos:expr, $a_pos:expr),
    ) => {
        paste! {
            impl $type {
                const A_MASK: $storage_type = ($type::MAX_A as $storage_type) << $a_pos;

                pub const fn with_alpha(
                    r: [<u $r_bits>],
                    g: [<u $g_bits>],
                    b: [<u $b_bits>],
                    a: [<u $a_bits>],
                ) -> Self {
                    Self(
                        (get_value!($r_bits, r) as $storage_type) << $r_pos
                        | (get_value!($g_bits, g) as $storage_type) << $g_pos
                        | (get_value!($b_bits, b) as $storage_type) << $b_pos
                        | (get_value!($a_bits, a) as $storage_type) << $a_pos
                    )
                }
            }
            impl RgbWithAlpha for $type {
                const MAX_A: u8 = get_value!($a_bits, [<u $a_bits>]::MAX);

                fn a(&self) -> u8 {
                    ((self.0 & Self::A_MASK) >> $a_pos) as u8
                }
            }
        }
    };
}

macro_rules! rgb_color {
    (
        $type:ident,
        $storage_type:ty,Rgb =
        ($r_bits:expr, $g_bits:expr, $b_bits:expr)
    ) => {
        impl_rgb_color!(
            $type,
            $storage_type,
            ($r_bits, $g_bits, $b_bits),
            ($g_bits + $b_bits, $b_bits, 0),
        );
        impl_debug_rgb!($type);
    };
    (
        $type:ident,
        $storage_type:ty,Rgba =
        ($r_bits:expr, $g_bits:expr, $b_bits:expr, $a_bits:expr)
    ) => {
        impl_rgb_color!(
            $type,
            $storage_type,
            ($r_bits, $g_bits, $b_bits),
            ($g_bits + $b_bits + $a_bits, $b_bits + $a_bits, $a_bits),
        );
        impl_rgb_alpha_channel!(
            $type,
            $storage_type,
            ($r_bits, $g_bits, $b_bits, $a_bits),
            ($g_bits + $b_bits + $a_bits, $b_bits + $a_bits, $a_bits, 0),
        );
        impl_debug_rgba!($type);
    };

    (
        $type:ident,
        $data_type:ty,
        $storage_type:ty,Bgr =
        ($r_bits:expr, $g_bits:expr, $b_bits:expr)
    ) => {
        impl_rgb_color!(
            $type,
            $data_type,
            $storage_type,
            ($r_bits, $g_bits, $b_bits),
            (0, $r_bits, $r_bits + $g_bits)
        );
        impl_debug_rgb!($type);
    };

    (
        $type:ident,
        $storage_type:ty,Bgra =
        ($r_bits:expr, $g_bits:expr, $b_bits:expr, $a_bits:expr)
    ) => {
        impl_rgb_color!(
            $type,
            $storage_type,
            ($r_bits, $g_bits, $b_bits),
            ($a_bits, $r_bits + $a_bits, $r_bits + $g_bits + $a_bits),
        );
        impl_rgb_alpha_channel!(
            $type,
            $storage_type,
            ($r_bits, $g_bits, $b_bits, $a_bits),
            ($a_bits, $r_bits + $a_bits, $r_bits + $g_bits + $a_bits, 0),
        );
        impl_debug_rgba!($type);
    };
}

/// Pixel color trait.
pub trait PixelColor: sealed::Sealed {}

/// RGB color.
pub trait RgbColor: PixelColor {
    const MAX_R: u8;
    const MAX_G: u8;
    const MAX_B: u8;
    const BLACK: Self;
    const RED: Self;
    const GREEN: Self;
    const BLUE: Self;
    const YELLOW: Self;
    const MAGENTA: Self;
    const CYAN: Self;
    const WHITE: Self;

    fn r(&self) -> u8;
    fn g(&self) -> u8;
    fn b(&self) -> u8;
}

/// RGB color with alpha channel.
pub trait RgbWithAlpha: RgbColor {
    const MAX_A: u8;

    fn a(&self) -> u8;
}

rgb_color!(Rgb565, u16, Rgb = (5, 6, 5));
//rgb_color!(Rgba8888, u32, Rgba = (8, 8, 8, 8));
//rgb_color!(Bgra8888, u32, Bgra = (8, 8, 8, 8));
