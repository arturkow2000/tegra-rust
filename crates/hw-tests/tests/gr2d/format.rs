//! Format tests which ensure that GR2D correctly lies out channels in memory.
//! These deliberately bypass color decoding through [`colorspace`] crate.
//!
//! Format names follow memory order, so e.g. ARGB8888 is `[A, R, G, B]` in memory.

use colorspace::{Abgr8888, Argb8888, color::Color};
use hw_tests::{
    PixelBufferExt as _,
    ctx::{Gr2d, gr2d as ctx},
    exec,
};

#[tegra_test_macros::test]
#[case::red(255, 0, 0, [0xff, 0xff, 0x00, 0x00])]
#[case::green(0, 255, 0, [0xff, 0x00, 0xff, 0x00])]
#[case::blue(0, 0, 255, [0xff, 0x00, 0x00, 0xff])]
async fn argb8888(
    ctx: &Gr2d,
    #[case] r: u8,
    #[case] g: u8,
    #[case] b: u8,
    #[case] expected: [u8; 4],
) {
    exec!(ctx, Argb8888, 1, 1, |buf| gr2d::clear_with_color(
        buf,
        Color::rgb(r, g, b)
    ))
    .for_each_raw_pixel(|_, _, px| assert_eq!(px, expected));
}

#[tegra_test_macros::test]
#[case::red(255, 0, 0, [0xff, 0x00, 0x00, 0xff])]
#[case::green(0, 255, 0, [0xff, 0x00, 0xff, 0x00])]
#[case::blue(0, 0, 255, [0xff, 0xff, 0x00, 0x00])]
async fn abgr8888(
    ctx: &Gr2d,
    #[case] r: u8,
    #[case] g: u8,
    #[case] b: u8,
    #[case] expected: [u8; 4],
) {
    exec!(ctx, Abgr8888, 1, 1, |buf| gr2d::clear_with_color(
        buf,
        Color::rgb(r, g, b)
    ))
    .for_each_raw_pixel(|_, _, px| assert_eq!(px, expected));
}
