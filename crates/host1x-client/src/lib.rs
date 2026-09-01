#![no_std]

use arbitrary_int::u10;

pub trait Client: 'static {
    const CLASS: u10;
}
