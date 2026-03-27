#![no_std]

use arbitrary_int::u10;

pub trait Client {
    const CLASS: u10;
}
