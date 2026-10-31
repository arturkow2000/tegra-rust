#![cfg_attr(not(test), no_std)]

pub mod color;
pub mod encoding;
mod format;
pub mod util;

pub use encoding::{
    BitsPerPixel, Channel, DefaultAlpha, Encoding, NoAlpha, Rgb, RgbAlpha, RgbGeneric,
};
pub use format::*;
