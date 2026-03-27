use arbitrary_int::traits::UnsignedInteger;

/// A 0-bit unsigned integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[allow(non_camel_case_types)]
pub struct u0;

/// Utility trait to allow storing either a real uint or zero-sized unit as field of a struct.
pub trait UnsignedIntegerOrUnit {
    const IS_REAL_INT: bool;
}
impl UnsignedIntegerOrUnit for u0 {
    const IS_REAL_INT: bool = false;
}
impl<T: UnsignedInteger> UnsignedIntegerOrUnit for T {
    const IS_REAL_INT: bool = true;
}
