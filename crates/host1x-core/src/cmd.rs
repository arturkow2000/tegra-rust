//! High-level, type-safe utilities for creating Host1x commands.

use core::{iter::FusedIterator, marker::PhantomData};

use arbitrary_int::{u4, u6, u10, u12};
use host1x_client::Client;
use host1x_reg::{Register, RegisterValue, reloc::Relocation};
use zerocopy::little_endian::U32 as U32LE;

#[cfg(doc)]
use gr2d_reg::{CTRL0, CTRL1, SBFMT0};

use crate::cmdseq::CommandSequence;

macro_rules! min_uint {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{
        let rest = min_uint!($($rest),+);
        let x = $x;
        // FIXME: currently we use UInt::value() because we can't compare non-standard
        // types without https://github.com/rust-lang/rust/issues/143802
        if x.value() < rest.value() { x } else { rest }
    }};
}

macro_rules! max_uint {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {{
        let rest = max_uint!($($rest),+);
        let x = $x;
        // FIXME: currently we use UInt::value() because we can't compare non-standard
        // types without https://github.com/rust-lang/rust/issues/143802
        if x.value() > rest.value() { x } else { rest }
    }};
}

macro_rules! mask_bits {
    ($typ:ty, $intyp:ty, $masklen:literal, $($R:ident),+) => {{
        let mut t: u32 = 0;
        let mut x = 0;
        let maxmask: u32 = (1 << $masklen) - 1;
        $(
            assert!($R::R::OFFSET.value() as u32 != x, "duplicated register offset");
            assert!($R::R::OFFSET.value() as u32 > x, "registers are not sorted");
            #[allow(unused)]
            { x = $R::R::OFFSET.value() as u32; }

            // FIXME: currently we use UInt::value() because we can't compare non-standard
            // types without https://github.com/rust-lang/rust/issues/143802
            let bit = 1 << (($R::R::OFFSET.value() - <Self as $typ>::REG_MIN.value()) as u32);
            // assert! requires string literal so we can't use const_format
            assert!(bit & maxmask == bit, concat!(stringify!($R), " can't fit into mask (too big distance beetween first and last register)"));
            t |= bit;
        )+
        t as $intyp
    }};
}

macro_rules! count_registers {
    ($($R:ident),+) => {
        0usize $(+ { let _ = stringify!($R); 1 })+
    };
}

macro_rules! count_relocations {
    ($($R:ident),+) => {
        0usize $(+ { let _ = stringify!($R); if $R::HAS_RELOCATION { 1 } else { 0 } })+
    };
}

/// Supported argument types for [`Mask`].
///
/// # Safety
/// - Implementors must ensure that [`MaskArgs::values`] yield exactly [`MaskArgs::N`] elements.
/// - Implementors must ensure that [`MaskArgs::relocs`] yield exactly [`MaskArgs::N_RELOCS`] elements.
pub unsafe trait MaskArgs<C: Client> {
    /// Register with the lowest address we are going to write. This will be set as
    /// offset part of the command.
    const REG_MIN: u12;

    /// Register with the highest address we are going to write.
    const REG_MAX: u12;

    /// The bitmask to be programmed to the GPU.
    const MASK: u16;

    /// The total number of registers we will write.
    const N: usize;

    /// The total number of relocations.
    const N_RELOCS: usize;

    /// Raw register values as understood by the hardware.
    fn values(&self) -> impl ExactSizeIterator<Item = u32>;

    /// High-level description of relocations to be performed prior to command submission.
    ///
    /// [`Relocation::patch_offset`] is relative to the first value.
    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation>;
}
macro_rules! impl_mask_args {
    ($(($R:ident, $idx:tt)),+) => {
        unsafe impl<C: Client, $($R: RegisterValue<C>),+> MaskArgs<C> for ($($R),+,) {
            const REG_MIN: u12 = min_uint!($($R::R::OFFSET),+);
            const REG_MAX: u12 = max_uint!($($R::R::OFFSET),+);
            const MASK: u16 = mask_bits!(MaskArgs<C>, u16, 16, $($R),+);
            const N: usize = count_registers!($($R),+);
            const N_RELOCS: usize = count_relocations!($($R),+);
            fn values(&self) -> impl ExactSizeIterator<Item = u32> {
                [
                    $(self.$idx.value()),+
                ].into_iter()
            }
            fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
                // RegisterValue trait is unsafe and implementors are required to
                // either Some or None, depending on HAS_RELOCATION. As long as
                // RegisterValue invariants hold, our will too.
                struct FilterExactSize<I> {
                    it: I,
                    size: usize
                }
                impl<I: Iterator<Item = Option<Relocation>>> Iterator for FilterExactSize<I> {
                    type Item = Relocation;

                    fn next(&mut self) -> Option<Self::Item> {
                        loop {
                            let v = self.it.next()?;
                            if let Some(v) = v {
                                self.size -= 1;
                                return Some(v);
                            }
                        }
                    }

                    fn size_hint(&self) -> (usize, Option<usize>) {
                        (self.size, Some(self.size))
                    }
                }
                impl<I: Iterator<Item = Option<Relocation>>> ExactSizeIterator for FilterExactSize<I> {
                    fn len(&self) -> usize {
                        self.size
                    }
                }
                impl<I: FusedIterator<Item = Option<Relocation>>> FusedIterator for FilterExactSize<I> {}
                FilterExactSize {
                    // Value of the n-th register is located n words after the first value.
                    it: [$(self.$idx.reloc().map(|mut r| { r.patch_offset += $idx; r })),+].into_iter(),
                    size: <Self as MaskArgs<C>>::N_RELOCS,
                }
            }
        }
    };
}
#[rustfmt::skip] impl_mask_args!((R0, 0));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14));
#[rustfmt::skip] impl_mask_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15));

/// Supported argument types for [`SetClass`].
///
/// # Safety
/// - Implementors must ensure that [`SetCLArgs::values`] yield exactly [`SetCLArgs::N`] elements.
/// - Implementors must ensure that [`SetCLArgs::relocs`] yield exactly [`SetCLArgs::N_RELOCS`] elements.
pub unsafe trait SetCLArgs<C: Client> {
    /// Register with the lowest address we are going to write. This will be set as
    /// offset part of the command.
    const REG_MIN: u12;

    /// Register with the highest address we are going to write.
    const REG_MAX: u12;

    /// The bitmask to be programmed to the GPU.
    const MASK: u6;

    /// The total number of registers we will write.
    const N: usize;

    /// The total number of relocations.
    const N_RELOCS: usize;

    /// Raw register values as understood by the hardware.
    fn values(&self) -> impl ExactSizeIterator<Item = u32>;

    /// High-level description of relocations to be performed prior to command submission.
    ///
    /// [`Relocation::patch_offset`] is relative to the first value.
    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation>;
}
macro_rules! impl_setcl_args {
    ($(($R:ident, $idx:tt)),+) => {
        unsafe impl<C: Client, $($R: RegisterValue<C>),+> SetCLArgs<C> for ($($R),+,) {
            const REG_MIN: u12 = min_uint!($($R::R::OFFSET),+);
            const REG_MAX: u12 = max_uint!($($R::R::OFFSET),+);
            const MASK: u6 = u6::new(mask_bits!(SetCLArgs<C>, u8, 6, $($R),+));
            const N: usize = count_registers!($($R),+);
            const N_RELOCS: usize = count_relocations!($($R),+);
            fn values(&self) -> impl ExactSizeIterator<Item = u32> {
                [
                    $(self.$idx.value()),+
                ].into_iter()
            }
            fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
                // RegisterValue trait is unsafe and implementors are required to
                // either Some or None, depending on HAS_RELOCATION. As long as
                // RegisterValue invariants hold, our will too.
                struct FilterExactSize<I> {
                    it: I,
                    size: usize
                }
                impl<I: Iterator<Item = Option<Relocation>>> Iterator for FilterExactSize<I> {
                    type Item = Relocation;

                    fn next(&mut self) -> Option<Self::Item> {
                        loop {
                            let v = self.it.next()?;
                            if let Some(v) = v {
                                self.size -= 1;
                                return Some(v);
                            }
                        }
                    }

                    fn size_hint(&self) -> (usize, Option<usize>) {
                        (self.size, Some(self.size))
                    }
                }
                impl<I: Iterator<Item = Option<Relocation>>> ExactSizeIterator for FilterExactSize<I> {
                    fn len(&self) -> usize {
                        self.size
                    }
                }
                impl<I: FusedIterator<Item = Option<Relocation>>> FusedIterator for FilterExactSize<I> {}
                FilterExactSize {
                    // Value of the n-th register is located n words after the first value.
                    it: [$(self.$idx.reloc().map(|mut r| { r.patch_offset += $idx; r })),+].into_iter(),
                    size: <Self as SetCLArgs<C>>::N_RELOCS,
                }
            }
        }
    };
}

impl_setcl_args!((R0, 0));
impl_setcl_args!((R0, 0), (R1, 1));
impl_setcl_args!((R0, 0), (R1, 1), (R2, 2));
impl_setcl_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3));
impl_setcl_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4));
impl_setcl_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5));

unsafe impl<C: Client> SetCLArgs<C> for () {
    const REG_MIN: u12 = u12::new(0);
    const REG_MAX: u12 = u12::new(0);
    const MASK: u6 = u6::new(0);
    const N: usize = 0;
    const N_RELOCS: usize = 0;

    fn values(&self) -> impl ExactSizeIterator<Item = u32> {
        [].into_iter()
    }

    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
        [].into_iter()
    }
}

/// Supported argument types for [`WriteIncr`].
///
/// # Safety
/// - Implementors must ensure that [`WriteIncrArgs::values`] yields exactly [`WriteIncrArgs::N`] elements.
/// - Implementors must ensure that [`WriteIncrArgs::relocs`] yields exactly [`WriteIncrArgs::N_RELOCS`] elements.
pub unsafe trait WriteIncrArgs<C: Client> {
    /// Register with the lowest address we are going to write. This will be set as
    /// offset part of the command.
    const REG_MIN: u12;

    /// The total number of registers we will write.
    const N: usize;

    /// The total number of relocations.
    const N_RELOCS: usize;

    /// Raw register values as understood by the hardware.
    fn values(&self) -> impl ExactSizeIterator<Item = u32>;

    /// High-level description of relocations to be performed prior to command submission.
    ///
    /// [`Relocation::patch_offset`] is relative to the first value.
    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation>;
}
macro_rules! impl_write_incr_args {
    ($(($R:ident, $idx:tt)),+) => {
        unsafe impl<C: Client, $($R: RegisterValue<C>),+> WriteIncrArgs<C> for ($($R),+,) {
            const REG_MIN: u12 = {
                let mut prev: Option<u12> = None;
                let mut off: Option<u12> = None;
                $(
                    if off.is_none() {
                        off = Some($R::R::OFFSET);
                    }
                    if let Some(prev) = prev {
                        assert!($R::R::OFFSET.value() as u32 != prev.value() as u32, "duplicated register offset");
                        assert!($R::R::OFFSET.value() as u32 > prev.value() as u32, "registers are not sorted");
                        assert!($R::R::OFFSET.value() as u32 == (prev.value() as u32) + 1, concat!(stringify!($R), " is not located directly after the previous register (consider using MASK command)"));
                    }
                    #[allow(unused)]
                    { prev = Some($R::R::OFFSET); }
                )+
                off.unwrap()
            };
            const N: usize = count_registers!($($R),+);
            const N_RELOCS: usize = count_relocations!($($R),+);
            fn values(&self) -> impl ExactSizeIterator<Item = u32> {
                [
                    $(self.$idx.value()),+
                ].into_iter()
            }
            fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
                // RegisterValue trait is unsafe and implementors are required to
                // return either Some or None, depending on HAS_RELOCATION. As long as
                // RegisterValue invariants hold, our will too.
                struct FilterExactSize<I> {
                    it: I,
                    size: usize
                }
                impl<I: Iterator<Item = Option<Relocation>>> Iterator for FilterExactSize<I> {
                    type Item = Relocation;

                    fn next(&mut self) -> Option<Self::Item> {
                        loop {
                            let v = self.it.next()?;
                            if let Some(v) = v {
                                self.size -= 1;
                                return Some(v);
                            }
                        }
                    }

                    fn size_hint(&self) -> (usize, Option<usize>) {
                        (self.size, Some(self.size))
                    }
                }
                impl<I: Iterator<Item = Option<Relocation>>> ExactSizeIterator for FilterExactSize<I> {
                    fn len(&self) -> usize {
                        self.size
                    }
                }
                impl<I: FusedIterator<Item = Option<Relocation>>> FusedIterator for FilterExactSize<I> {}
                FilterExactSize {
                    // Value of the n-th register is located n words after the first value.
                    it: [$(self.$idx.reloc().map(|mut r| { r.patch_offset += $idx; r })),+].into_iter(),
                    size: <Self as WriteIncrArgs<C>>::N_RELOCS,
                }
            }
        }
    };
}

// Note: WriteIncr documentations says how many registers may write. Need to keep that in mind
// when extending support for larger tuples.
#[rustfmt::skip] impl_write_incr_args!((R0, 0));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24), (R25, 25));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24), (R25, 25), (R26, 26));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24), (R25, 25), (R26, 26), (R27, 27));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24), (R25, 25), (R26, 26), (R27, 27), (R28, 28));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24), (R25, 25), (R26, 26), (R27, 27), (R28, 28), (R29, 29));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24), (R25, 25), (R26, 26), (R27, 27), (R28, 28), (R29, 29), (R30, 30));
#[rustfmt::skip] impl_write_incr_args!((R0, 0), (R1, 1), (R2, 2), (R3, 3), (R4, 4), (R5, 5), (R6, 6), (R7, 7), (R8, 8), (R9, 9), (R10, 10), (R11, 11), (R12, 12), (R13, 13), (R14, 14), (R15, 15), (R16, 16), (R17, 17), (R18, 18), (R19, 19), (R20, 20), (R21, 21), (R22, 22), (R23, 23), (R24, 24), (R25, 25), (R26, 26), (R27, 27), (R28, 28), (R29, 29), (R30, 30), (R31, 31));

const OPCODE_SETCL: u4 = u4::new(0);
const OPCODE_INCR: u4 = u4::new(1);
const OPCODE_NONINCR: u4 = u4::new(2);
const OPCODE_MASK: u4 = u4::new(3);

const fn op_setcl(classid: u10, offset: u12, mask: u6) -> u32 {
    ((OPCODE_SETCL.value() as u32) << 28)
        | ((offset.value() as u32) << 16)
        | ((classid.value() as u32) << 6)
        | (mask.value() as u32)
}

const fn op_incr(offset: u12, count: u16) -> u32 {
    ((OPCODE_INCR.value() as u32) << 28) | ((offset.value() as u32) << 16) | (count as u32)
}

const fn op_nonincr(offset: u12, count: u16) -> u32 {
    ((OPCODE_NONINCR.value() as u32) << 28) | ((offset.value() as u32) << 16) | (count as u32)
}

const fn op_mask(offset: u12, mask: u16) -> u32 {
    ((OPCODE_MASK.value() as u32) << 28) | ((offset.value() as u32) << 16) | (mask as u32)
}

/// Host1x SetCL command.
///
/// SetCL command chooses destination hardware block (e.g. GR2D, GR3D) that receives our commands.
///
/// Optionally may write up to 6 registers according to a bitmask - this feature works
/// similar to the [`Mask`] command, however the bitmask is 6-bit instead of 16-bit, as
/// 10 upper bits of mask have been repurposed as class ID field.
///
/// # Examples
///
/// When using SetCL just to set target class ID (without writing any regiters)
/// there is no need to specify generics:
///
/// ```
/// # use host1x_core::cmd::SetClass;
/// # use gr2d_reg::SBFMT0;
/// # use arbitrary_int::u10;
/// SetClass::<_, _, gr2d_reg::Client, _>::new();
/// ```
///
/// Otherwise similar constraints as with [`Mask`] command apply, with one notable
/// exception that the mask part of SetCL command is 6-bit (instead of 16-bit), so
/// the following is possible with [`Mask`], but not with [`SetClass`]:
///
/// ```compile_fail,E0080
/// # use host1x_core::cmd::SetClass;
/// # use gr2d_reg::{CLIPLEFTTOP, SBFMT0};
/// SetClass::<2, 0, gr2d_reg::Client, _>::with_register_write((SBFMT0::raw(0xdeadbeef), CLIPLEFTTOP::raw(0xc0de1ed0))); // assertion fail!
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SetClass<const N: usize, const N_RELOCS: usize, C: Client, Args: SetCLArgs<C>> {
    first_word: U32LE,
    args: [U32LE; N],
    relocs: [Relocation; N_RELOCS],
    _phantom: PhantomData<(C, Args)>,
}
impl<C: Client> SetClass<0, 0, C, ()> {
    #[inline]
    pub const fn new() -> Self {
        Self::with_raw_class(C::CLASS)
    }

    #[inline]
    const fn with_raw_class(classid: u10) -> Self {
        Self {
            first_word: U32LE::new(op_setcl(classid, u12::new(0), u6::new(0))),
            args: [],
            relocs: [],
            _phantom: PhantomData,
        }
    }
}
impl<C: Client> Default for SetClass<0, 0, C, ()> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const N: usize, const N_RELOCS: usize, C: Client, Args: SetCLArgs<C>>
    SetClass<N, N_RELOCS, C, Args>
{
    // Can't be located in constructor due to generic args.
    const _ASSERT_N_EQ_ARGS_N: () = assert!(N == Args::N);
    const _ASSERT_N_RELOCS_EQ_ARGS_N_RELOCS: () = assert!(N_RELOCS == Args::N_RELOCS);

    // TODO: make this const?
    // at minimum we need https://github.com/rust-lang/rust/issues/92476
    #[inline]
    pub fn with_raw_class_and_register_write(classid: u10, values: Args) -> Self {
        // Force evaluation, unused constants are not evaluated, so assertion can't be triggered.
        #[allow(clippy::let_unit_value)]
        let _ = Self::_ASSERT_N_EQ_ARGS_N;
        #[allow(clippy::let_unit_value)]
        let _ = Self::_ASSERT_N_RELOCS_EQ_ARGS_N_RELOCS;

        let mut args = const { [U32LE::new(0); N] };
        let mut relocs = const { [zero_reloc(); N_RELOCS] };
        let mut old_relocs = values.relocs();
        let mut values = values.values();

        // Just to be sure, won't happen as long as `SetCLArgs` implementation is correct.
        debug_assert_eq!(values.len(), N);

        // FIXME: on little-endian should compile down to simple memory copy, why it doesn't?
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            // Safety: we checked that there are exactly N values.
            unsafe { *args.get_unchecked_mut(i) = U32LE::new(values.next().unwrap_unchecked()) }
        }

        #[allow(clippy::needless_range_loop)]
        for i in 0..N_RELOCS {
            // Safety: we checked that there are exactly N_RELOCS values.
            unsafe {
                let old = old_relocs.next().unwrap_unchecked();
                *relocs.get_unchecked_mut(i) = Relocation {
                    // Skip the opcode word.
                    patch_offset: old.patch_offset + 1,
                    mapping: old.mapping,
                    buf_offset: old.buf_offset,
                    shift: old.shift,
                };
            }
        }

        Self {
            first_word: U32LE::new(op_setcl(classid, Args::REG_MIN, Args::MASK)),
            args,
            relocs,
            _phantom: PhantomData,
        }
    }
    pub fn with_register_write(values: Args) -> Self {
        Self::with_raw_class_and_register_write(C::CLASS, values)
    }
}

unsafe impl<const N: usize, const N_RELOCS: usize, C: Client, Args: SetCLArgs<C>> CommandSequence<C>
    for SetClass<N, N_RELOCS, C, Args>
{
    const N_WORDS: usize = N + 1;
    const N_RELOCS: usize = Args::N_RELOCS;

    fn words(&self) -> impl ExactSizeIterator<Item = U32LE> {
        // Safety:
        // - first_word is an U32LE, followed by an array [U32LE; N]
        // - self has #[repr(C)], first_word is at offset 0 and U32LE has alignment of 1, so
        //   there is no padding before args
        // - the pointer is derived from self, not from &self.first_word, so it may access
        //   args too
        unsafe { core::slice::from_raw_parts(core::ptr::from_ref(self).cast::<U32LE>(), N + 1) }
            .iter()
            .copied()
    }

    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
        self.relocs.into_iter()
    }
}

/// Host1x INCR command.
///
/// Allows to write up to 32 (hardware allows up to 65535, but this is not supported
/// currently due to to rust limitations) 32-bit words, incrementing offset by 1 (word)
/// after each write. Essentially, this command allows to write multiple
/// registers.
///
/// # Examples
///
/// ```
/// # use host1x_core::cmd::WriteIncr;
/// # use gr2d_reg::{CTRL0, CTRL1, SBFMT0};
/// WriteIncr::<1, 0, gr2d_reg::Client, _>::new((SBFMT0::SIFMT::B8G8R8A8 + SBFMT0::DIFMT::R8G8B8A8, ));
/// WriteIncr::<3, 0, gr2d_reg::Client, _>::new((SBFMT0::SIFMT::B8G8R8A8 + SBFMT0::DIFMT::R8G8B8A8, CTRL0::IMODE::MULTIPLEX, CTRL1::FR_TYPE::ROT_270 + CTRL1::FR_MODE::SRC_DST_COPY));
/// ```
///
/// Compiler checks that whether the command can actually be built, so for example
/// attempting to write [`SBFMT0`] directly followed by [`CTRL1`] will fail at compile time
/// (beetwen these two there is [`CTRL0`]):
///
/// ```compile_fail,E0080
/// # use host1x_core::cmd::WriteIncr;
/// # use gr2d_reg::{CTRL1, SBFMT0};
/// WriteIncr::<2, 0, gr2d_reg::Client, _>::new((SBFMT0::SIFMT::B8G8R8A8 + SBFMT0::DIFMT::R8G8B8A8, CTRL1::FR_TYPE::ROT_270 + CTRL1::FR_MODE::SRC_DST_COPY));
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteIncr<const N: usize, const N_RELOCS: usize, C: Client, Args: WriteIncrArgs<C>> {
    first_word: U32LE,
    // FIXME: Not possible to do `args: [u32; Args::N]` without
    // https://github.com/rust-lang/rust/issues/76560
    // constructor ensures N matches with Args::N
    //
    // FIXME: Without #76560 we can't even do `args: [U32LE; N + 1]`, so first word needs
    // to be kept as separate variable. In effect that requires unsafe to cast to U32LE slice.
    args: [U32LE; N],
    relocs: [Relocation; N_RELOCS],
    _phantom: PhantomData<(C, Args)>,
}

impl<const N: usize, const N_RELOCS: usize, C: Client, Args: WriteIncrArgs<C>>
    WriteIncr<N, N_RELOCS, C, Args>
{
    // Can't be located in constructor due to generic args.
    const _ASSERT_N_EQ_ARGS_N: () = assert!(N == Args::N);
    const _ASSERT_N_NOT_TOO_BIG: () = assert!(N & 0xffff == N);
    const _ASSERT_N_RELOCS_EQ_ARGS_N_RELOCS: () = assert!(N_RELOCS == Args::N_RELOCS);

    // TODO: make this const?
    // at minimum we need https://github.com/rust-lang/rust/issues/92476
    #[inline]
    pub fn new(values: Args) -> Self {
        // Force evaluation, unused constants are not evaluated, so assertion can't be triggered.
        #[allow(clippy::let_unit_value)]
        let _ = Self::_ASSERT_N_EQ_ARGS_N;
        #[allow(clippy::let_unit_value)]
        let _ = Self::_ASSERT_N_NOT_TOO_BIG;
        #[allow(clippy::let_unit_value)]
        let _ = Self::_ASSERT_N_RELOCS_EQ_ARGS_N_RELOCS;

        let mut args = const { [U32LE::new(0); N] };
        let mut relocs = const { [zero_reloc(); N_RELOCS] };
        let mut old_relocs = values.relocs();
        let mut values = values.values();

        // Just to be sure, won't happen as long as `WriteIncrArgs` implementation is correct.
        debug_assert_eq!(values.len(), N);

        // FIXME: on little-endian should compile down to simple memory copy, why it doesn't?
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            // Safety: we checked that there are exactly N values.
            unsafe { *args.get_unchecked_mut(i) = U32LE::new(values.next().unwrap_unchecked()) }
        }

        #[allow(clippy::needless_range_loop)]
        for i in 0..N_RELOCS {
            // Safety: we checked that there are exactly N_RELOCS values.
            unsafe {
                let old = old_relocs.next().unwrap_unchecked();
                *relocs.get_unchecked_mut(i) = Relocation {
                    // Skip the opcode word.
                    patch_offset: old.patch_offset + 1,
                    mapping: old.mapping,
                    buf_offset: old.buf_offset,
                    shift: old.shift,
                };
            }
        }

        Self {
            first_word: U32LE::new(op_incr(Args::REG_MIN, Args::N as u16)),
            args,
            relocs,
            _phantom: PhantomData,
        }
    }
}

unsafe impl<const N: usize, const N_RELOCS: usize, C: Client, Args: WriteIncrArgs<C>>
    CommandSequence<C> for WriteIncr<N, N_RELOCS, C, Args>
{
    const N_WORDS: usize = N + 1;
    const N_RELOCS: usize = N_RELOCS;

    fn words(&self) -> impl ExactSizeIterator<Item = U32LE> {
        // Safety:
        // - first_word is an U32LE, followed by an array [U32LE; N]
        // - self has #[repr(C)], first_word is at offset 0 and U32LE has alignment of 1, so
        //   there is no padding before args
        // - the pointer is derived from self, not from &self.first_word, so it may access
        //   args too
        unsafe { core::slice::from_raw_parts(core::ptr::from_ref(self).cast::<U32LE>(), N + 1) }
            .iter()
            .copied()
    }

    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
        self.relocs.into_iter()
    }
}

/// Host1x NOINCR command.
///
/// NOINCR allows to write up to 65535 32-bit words at a fixed offset.
///
/// Writing relocatable registers is not supported, and is rejected at compile
/// time:
///
/// ```compile_fail,E0080
/// # use host1x_core::cmd::WriteNoIncr;
/// # use gr2d_reg::DSTBA;
/// WriteNoIncr::new([unsafe { DSTBA::for_raw_mapping::<'static, gr2d_reg::Client>(1, 0, 0) }]);
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WriteNoIncr<const N: usize, C: Client, Reg: Register<C>> {
    first_word: U32LE,
    args: [U32LE; N],
    _phantom: PhantomData<(C, Reg)>,
}

impl<const N: usize, C: Client, Reg: Register<C>> WriteNoIncr<N, C, Reg> {
    // TODO: with https://github.com/rust-lang/rust/issues/143773
    // we should be able to just do `N.try_into().unwrap()`
    const N_U16: u16 = {
        let t = N as u16;
        assert!(N & 0xffff == N, "N is too big");
        t
    };

    #[inline]
    pub fn new<V: RegisterValue<C, R = Reg>>(values: [V; N]) -> Self {
        // Relocations are not emitted, a relocatable value would end up as a
        // null address.
        const {
            assert!(
                !V::HAS_RELOCATION,
                "WriteNoIncr doesn't support relocatable registers"
            )
        };

        let mut args = [U32LE::new(0); N];

        // FIXME: on little-endian should compile down to simple memory copy, why it doesn't?
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            // Safety: method signature enforces there are exactly N values.
            unsafe { *args.get_unchecked_mut(i) = U32LE::new(values[i].value()) }
        }

        Self {
            first_word: U32LE::new(op_nonincr(Reg::OFFSET, Self::N_U16)),
            args,
            _phantom: PhantomData,
        }
    }
}

unsafe impl<const N: usize, C: Client, Reg: Register<C>> CommandSequence<C>
    for WriteNoIncr<N, C, Reg>
{
    const N_WORDS: usize = N + 1;
    // Relocatable values are rejected by the constructor.
    const N_RELOCS: usize = 0;

    fn words(&self) -> impl ExactSizeIterator<Item = U32LE> {
        // Safety:
        // - first_word is an U32LE, followed by an array [U32LE; N]
        // - self has #[repr(C)], first_word is at offset 0 and U32LE has alignment of 1, so
        //   there is no padding before args
        // - the pointer is derived from self, not from &self.first_word, so it may access
        //   args too
        unsafe { core::slice::from_raw_parts(core::ptr::from_ref(self).cast::<U32LE>(), N + 1) }
            .iter()
            .copied()
    }

    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
        [].into_iter()
    }
}

/// Host1x MASK command.
///
/// Can write up to 16 registers starting at any given offset. Currently, due to
/// Rust type system limitations, requires manually specifying number of registers
/// we want write, in addition to correct number of values. Current implementation
/// asserts at compile time that this matches.
///
/// # Examples
///
/// Currently write of single register must be done by giving a 1-element tuple
/// to the constructor:
///
/// ```
/// # use host1x_core::cmd::Mask;
/// # use gr2d_reg::SBFMT0;
/// Mask::<1, 0, gr2d_reg::Client, _>::new((SBFMT0::raw(0xdeadbeef),));
/// ```
///
/// In similar manner, one can create MASK cmd for up to 16 registers.
///
/// ```
/// # use host1x_core::cmd::Mask;
/// # use gr2d_reg::{CTRL0, SBFMT0};
/// Mask::<2, 0, gr2d_reg::Client, _>::new((SBFMT0::raw(0xdeadbeef), CTRL0::raw(0xc0de1ed0)));
/// ```
///
/// Registers must be close-enough to each other so they can fit in the mask.
/// This is asserted at build time:
///
/// ```compile_fail,E0080
/// # use host1x_core::cmd::Mask;
/// # use gr2d_reg::{SBFMT0, DSTBA_B};
/// Mask::<2, 0, gr2d_reg::Client, _>::new((SBFMT0::raw(0xdeadbeef), unsafe { DSTBA_B::for_raw_mapping(1, 0, 0) })); // assertion fail!
/// ```
///
/// Registers must be sorted in the same order they will be written by the Host1x hardware,
/// that is starting with lowest register offset, ascending order:
///
/// ```compile_fail,E0080
/// # use host1x_core::cmd::Mask;
/// # use gr2d_reg::{CTRL0, SBFMT0};
/// Mask::<2, 0, gr2d_reg::Client, _>::new((CTRL0::raw(0xdeadbeef), SBFMT0::raw(0xdeadc0de))); // assertion fail!
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Mask<const N: usize, const N_RELOCS: usize, C: Client, Args: MaskArgs<C>> {
    first_word: U32LE,
    // FIXME: Not possible to do `args: [u32; Args::N]` without
    // https://github.com/rust-lang/rust/issues/76560
    // constructor ensures N matches with Args::N
    //
    // FIXME: Without #76560 we can't even do `args: [U32LE; N + 1]`, so first word needs
    // to be kept as separate variable. In effect that requires unsafe to cast to U32LE slice.
    args: [U32LE; N],
    relocs: [Relocation; N_RELOCS],
    _phantom: PhantomData<(C, Args)>,
}

impl<const N: usize, const N_RELOCS: usize, C: Client, Args: MaskArgs<C>>
    Mask<N, N_RELOCS, C, Args>
{
    // Can't be located in constructor due to generic args.
    const _ASSERT_N_EQ_ARGS_N: () = assert!(N == Args::N);
    const _ASSERT_N_RELOCS_EQ_ARGS_N_RELOCS: () = assert!(N_RELOCS == Args::N_RELOCS);

    // TODO: make this const?
    // at minimum we need https://github.com/rust-lang/rust/issues/92476
    #[inline]
    pub fn new(values: Args) -> Self {
        // Force evaluation, unused constants are not evaluated, so assertion can't be triggered.
        #[allow(clippy::let_unit_value)]
        let _ = Self::_ASSERT_N_EQ_ARGS_N;
        #[allow(clippy::let_unit_value)]
        let _ = Self::_ASSERT_N_RELOCS_EQ_ARGS_N_RELOCS;

        let mut args = const { [U32LE::new(0); N] };
        let mut relocs = const { [zero_reloc(); N_RELOCS] };
        let mut old_relocs = values.relocs();
        let mut values = values.values();

        // Just to be sure, won't happen as long as `MaskArgs` implementation is correct.
        debug_assert_eq!(values.len(), N);
        debug_assert_eq!(old_relocs.len(), N_RELOCS);

        // FIXME: on little-endian should compile down to simple memory copy, why it doesn't?
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            // Safety: we checked that there are exactly N values.
            unsafe { *args.get_unchecked_mut(i) = U32LE::new(values.next().unwrap_unchecked()) }
        }

        #[allow(clippy::needless_range_loop)]
        for i in 0..N_RELOCS {
            // Safety: we checked that there are exactly N_RELOCS values.
            unsafe {
                let old = old_relocs.next().unwrap_unchecked();
                *relocs.get_unchecked_mut(i) = Relocation {
                    // Skip the opcode word.
                    patch_offset: old.patch_offset + 1,
                    mapping: old.mapping,
                    buf_offset: old.buf_offset,
                    shift: old.shift,
                };
            }
        }

        Self {
            first_word: U32LE::new(op_mask(Args::REG_MIN, Args::MASK)),
            args,
            relocs,
            _phantom: PhantomData,
        }
    }
}

unsafe impl<const N: usize, const N_RELOCS: usize, C: Client, Args: MaskArgs<C>> CommandSequence<C>
    for Mask<N, N_RELOCS, C, Args>
{
    const N_WORDS: usize = N + 1;
    const N_RELOCS: usize = N_RELOCS;

    fn words(&self) -> impl ExactSizeIterator<Item = U32LE> {
        // Safety:
        // - first_word is an U32LE, followed by an array [U32LE; N]
        // - self has #[repr(C)], first_word is at offset 0 and U32LE has alignment of 1, so
        //   there is no padding before args
        // - the pointer is derived from self, not from &self.first_word, so it may access
        //   args too
        unsafe { core::slice::from_raw_parts(core::ptr::from_ref(self).cast::<U32LE>(), N + 1) }
            .iter()
            .copied()
    }

    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
        self.relocs.into_iter()
    }
}

const fn zero_reloc() -> Relocation {
    Relocation {
        patch_offset: 0,
        mapping: 0,
        buf_offset: 0,
        shift: 0,
    }
}

#[cfg(test)]
mod tests {
    use arbitrary_int::{u6, u10, u12};
    use gr2d_reg::{
        ALPHABLEND, CTRL0, CTRL1, CTRLMAIN, Client as Gr2d, DSTBA, DSTBA_B, DSTST, PATKEY, PATOS,
        SBFMT0,
    };
    use zerocopy::little_endian::U32 as U32LE;

    use crate::{
        cmdseq::CommandSequence,
        reg::{Register, reloc::Relocation},
    };

    use super::{Mask, SetClass, WriteIncr, WriteNoIncr, op_mask, op_nonincr, op_setcl};

    macro_rules! reg_off {
        ($reg:ident) => {
            <$reg::Register as Register<Gr2d>>::OFFSET
        };
    }

    #[test]
    fn test_setcl_gen() {
        let words: Vec<u32> = SetClass::<_, _, Gr2d, _>::new()
            .words()
            .map(U32LE::get)
            .collect();
        assert_eq!(words, [op_setcl(u10::new(0x51), u12::new(0), u6::new(0))]);
        let words: Vec<u32> = SetClass::<5, 0, Gr2d, _>::with_register_write((
            SBFMT0::SIFMT::B8G8R8A8 + SBFMT0::DIFMT::R8G8B8A8,
            CTRL0::IMODE::MULTIPLEX,
            CTRL1::zero(),
            CTRLMAIN::zero(),
            ALPHABLEND::ALPHA.val(172),
        ))
        .words()
        .map(U32LE::get)
        .collect();
        assert_eq!(
            words,
            [
                op_setcl(u10::new(0x51), u12::new(0x1c), u6::new(0b101111)),
                0x0e0f,
                0x00,
                0x00,
                0x00,
                0xac,
            ]
        );
    }

    #[test]
    fn test_nonincr_gen() {
        let words: Vec<u32> = WriteNoIncr::new([
            SBFMT0::SIFMT::B5G6R5 + SBFMT0::DIFMT::B5G6R5,
            SBFMT0::raw(0x42),
        ])
        .words()
        .map(U32LE::get)
        .collect();
        assert_eq!(words, [op_nonincr(u12::new(0x1c), 2), 0x0808, 0x42]);
    }

    #[test]
    fn test_mask_gen() {
        let words: Vec<u32> = Mask::<1, 0, Gr2d, _>::new((SBFMT0::raw(0xdeadbeef),))
            .words()
            .map(U32LE::get)
            .collect();
        assert_eq!(words, [op_mask(reg_off!(SBFMT0), 0b1), 0xdeadbeef]);

        let words: Vec<u32> =
            Mask::<2, 0, Gr2d, _>::new((SBFMT0::raw(0xdeadbeef), CTRL0::raw(0xc0de1ed0)))
                .words()
                .map(U32LE::get)
                .collect();
        assert_eq!(
            words,
            [op_mask(reg_off!(SBFMT0), 0b11), 0xdeadbeef, 0xc0de1ed0]
        );
    }

    /// Relocations with given (patch_offset, mapping) pairs.
    fn relocs<const N: usize>(relocs: [(u32, u32); N]) -> Vec<Relocation> {
        relocs
            .map(|(patch_offset, mapping)| Relocation {
                patch_offset,
                mapping,
                buf_offset: 0,
                shift: 0,
            })
            .to_vec()
    }

    #[test]
    fn test_mask_relocs() {
        let cmd = Mask::<3, 1, Gr2d, _>::new((
            PATOS::raw(0x1111),
            unsafe { DSTBA::for_raw_mapping::<'static, Gr2d>(5, 0, 0) },
            DSTST::raw(0x2222),
        ));
        let words: Vec<u32> = cmd.words().map(U32LE::get).collect();
        assert_eq!(words[1..], [0x1111, 0, 0x2222]);
        assert_eq!(cmd.relocs().collect::<Vec<_>>(), relocs([(2, 5)]));
    }

    #[test]
    fn test_setcl_relocs() {
        let cmd = SetClass::<2, 1, Gr2d, _>::with_register_write((PATOS::raw(0x1111), unsafe {
            DSTBA::for_raw_mapping::<'static, Gr2d>(5, 0, 0)
        }));
        let words: Vec<u32> = cmd.words().map(U32LE::get).collect();
        assert_eq!(words[1..], [0x1111, 0]);
        assert_eq!(cmd.relocs().collect::<Vec<_>>(), relocs([(2, 5)]));
    }

    #[test]
    fn test_incr_relocs() {
        let cmd = WriteIncr::<3, 2, Gr2d, _>::new((
            PATKEY::raw(0x1111),
            unsafe { DSTBA::for_raw_mapping::<'static, Gr2d>(5, 0, 0) },
            unsafe { DSTBA_B::for_raw_mapping::<'static, Gr2d>(6, 0, 0) },
        ));
        let words: Vec<u32> = cmd.words().map(U32LE::get).collect();
        assert_eq!(words[1..], [0x1111, 0, 0]);
        assert_eq!(cmd.relocs().collect::<Vec<_>>(), relocs([(2, 5), (3, 6)]));
    }
}
