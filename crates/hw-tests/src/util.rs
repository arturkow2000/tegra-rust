//! Small helpers shared by the tests.

use embedded_graphics::{
    prelude::{Point, Size},
    primitives::Rectangle,
};

/// A `w`×`h` rectangle with its top-left corner at (`x`, `y`).
pub fn rect(x: i32, y: i32, w: u32, h: u32) -> Rectangle {
    Rectangle {
        top_left: Point::new(x, y),
        size: Size::new(w, h),
    }
}
