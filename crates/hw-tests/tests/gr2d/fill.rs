//! GR2D fill and clipping tests running on real hardware.

use colorspace::{NoAlpha, RgbGeneric, color::Color};
use embedded_graphics::{prelude::Point, primitives::Rectangle};
use hw_tests::{
    PixelBufferExt as _, RgbFormat,
    ctx::{Gr2d, gr2d as ctx},
    exec,
    util::rect,
};

const CLIP_BUF_SIZE: u32 = 8;

#[tegra_test_macros::test]
#[case::red(255, 0, 0)]
#[case::green(0, 255, 0)]
#[case::blue(0, 0, 255)]
#[case::white(255, 255, 255)]
#[case::black(0, 0, 0)]
async fn clear<F: RgbFormat>(ctx: &Gr2d, #[case] r: u8, #[case] g: u8, #[case] b: u8) {
    let color = Color::<F::Encoding>::rgb(r, g, b);
    exec!(ctx, F, 1, 1, |buf| gr2d::clear_with_color(buf, color))
        .for_each_pixel(|x, y, got| assert_eq!(got, color, "pixel ({x}, {y})"));
}

#[tegra_test_macros::test]
#[case::inside(rect(2, 2, 4, 4))]
#[case::single_pixel(rect(0, 0, 1, 1))]
#[case::whole_buffer(rect(0, 0, 8, 8))]
#[case::larger_than_buffer(rect(0, 0, 16, 16))]
#[case::partly_outside(rect(6, 6, 4, 4))]
#[case::fully_outside(rect(8, 0, 4, 4))]
async fn fill_rect_clipped<F: RgbFormat>(ctx: &Gr2d, #[case] rect: Rectangle) {
    let bg = Color::<F::Encoding>::rgb(0, 0, 255);
    let fg = Color::<F::Encoding>::rgb(0, 255, 0);
    exec!(
        ctx,
        F,
        CLIP_BUF_SIZE,
        CLIP_BUF_SIZE,
        (
            |buf| gr2d::clear_with_color(buf, bg),
            |buf| gr2d::fill_rect(buf, Color::<Rgb8>::rgb(0, 255, 0), rect),
        )
    )
    .for_each_pixel(|x, y, got| {
        let covered = rect.contains(Point::new(x as i32, y as i32));
        let want = if covered { fg } else { bg };
        assert_eq!(got, want, "pixel ({x}, {y})");
    });
}

type Rgb8 = RgbGeneric<u8, u8, u8, NoAlpha>;
type Rgba8 = RgbGeneric<u8, u8, u8, u8>;

/// Colors without alpha replace the pixels, even if the buffer has alpha.
#[tegra_test_macros::test]
async fn fill_rect_no_alpha<F: RgbFormat>(ctx: &Gr2d) {
    let bg = Color::<F::Encoding>::rgb(0, 0, 255);
    let rect = rect(2, 2, 4, 4);
    exec!(
        ctx,
        F,
        CLIP_BUF_SIZE,
        CLIP_BUF_SIZE,
        (
            |buf| gr2d::clear_with_color(buf, bg),
            |buf| gr2d::fill_rect(buf, Color::<Rgb8>::rgb(0, 255, 0), rect),
        )
    )
    .for_each_pixel(|x, y, got| {
        let covered = rect.contains(Point::new(x as i32, y as i32));
        let want = if covered {
            Color::<F::Encoding>::rgb(0, 255, 0)
        } else {
            bg
        };
        assert_eq!(got, want, "pixel ({x}, {y})");
    });
}

/// Colors with alpha are blended onto the pixels, unless fully opaque.
///
/// Only checks RGB channels: what blending does to the buffer's alpha channel
/// isn't specified yet.
#[tegra_test_macros::test]
#[case::transparent(0x00)]
#[case::quarter(0x40)]
#[case::half(0x80)]
#[case::three_quarters(0xc0)]
#[case::almost_opaque(0xfe)]
#[case::opaque(0xff)]
async fn fill_rect_alpha<F: RgbFormat>(ctx: &Gr2d, #[case] alpha: u8) {
    let bg = Color::<F::Encoding>::rgb(0, 0, 255);
    let rect = rect(2, 2, 4, 4);
    exec!(
        ctx,
        F,
        CLIP_BUF_SIZE,
        CLIP_BUF_SIZE,
        (
            |buf| gr2d::clear_with_color(buf, bg),
            |buf| gr2d::fill_rect(buf, Color::<Rgba8>::rgba(255, 0, 0, alpha), rect),
        )
    )
    .for_each_pixel(|x, y, got| {
        let covered = rect.contains(Point::new(x as i32, y as i32));
        if !covered {
            assert_eq!(got, bg, "pixel ({x}, {y})");
        } else if alpha == 0xff {
            assert_eq!(got, Color::rgb(255, 0, 0), "pixel ({x}, {y})");
        } else {
            // Red blends from 0 and blue from 255, allow off-by-one rounding.
            let (r, g, b) = (got.r(), got.g(), got.b());
            assert!(
                r.abs_diff(alpha) <= 1 && g == 0 && b.abs_diff(!alpha) <= 1,
                "pixel ({x}, {y}): got {got:?}, want ~(r: {alpha}, g: 0, b: {})",
                !alpha
            );
        }
    });
}
