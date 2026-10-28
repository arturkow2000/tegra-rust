//! Format tests which ensure that GR2D correctly lies out channels in memory.
//! These deliberately bypass color decoding through [`colorspace`] crate.
//!
//! Format names follow Linux DRM convention, so e.g. RGBA8888 becomes [A, B, G, R]
//! in memory, BGRA8888 [A, R, G, B], etc.

use colorspace::{Bgra8888, Rgba8888, color::Color};
use hw_tests::{
    PixelBufferExt as _,
    ctx::{Gr2d, gr2d as ctx},
    exec,
};

/// BGRA8888 in memory is `[A, R, G, B]`.
#[tegra_test_macros::test]
#[case::red(255, 0, 0, [0xff, 0xff, 0x00, 0x00])]
#[case::green(0, 255, 0, [0xff, 0x00, 0xff, 0x00])]
#[case::blue(0, 0, 255, [0xff, 0x00, 0x00, 0xff])]
async fn bgra8888(
    ctx: &Gr2d,
    #[case] r: u8,
    #[case] g: u8,
    #[case] b: u8,
    #[case] expected: [u8; 4],
) {
    exec!(ctx, Bgra8888, 1, 1, |buf| gr2d::clear_with_color(
        buf,
        Color::rgb(r, g, b)
    ))
    .for_each_raw_pixel(|_, _, px| assert_eq!(px, expected));
}

/// RGBA8888 in memory is `[A, B, G, R]`.
#[tegra_test_macros::test]
#[case::red(255, 0, 0, [0xff, 0x00, 0x00, 0xff])]
#[case::green(0, 255, 0, [0xff, 0x00, 0xff, 0x00])]
#[case::blue(0, 0, 255, [0xff, 0xff, 0x00, 0x00])]
async fn rgba8888(
    ctx: &Gr2d,
    #[case] r: u8,
    #[case] g: u8,
    #[case] b: u8,
    #[case] expected: [u8; 4],
) {
    exec!(ctx, Rgba8888, 1, 1, |buf| gr2d::clear_with_color(
        buf,
        Color::rgb(r, g, b)
    ))
    .for_each_raw_pixel(|_, _, px| assert_eq!(px, expected));
}
