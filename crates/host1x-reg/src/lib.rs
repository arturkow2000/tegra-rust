#![no_std]
//! Utilities for defining device registers accessible through Host1x interface.

pub mod class;
pub mod reloc;
#[doc(hidden)]
pub mod __reexports {
    pub use ::arbitrary_int;
    pub use ::host1x_buffer;
    pub use ::host1x_client;
    pub use ::tock_registers;
}

use arbitrary_int::u12;
use host1x_client::Client;
use tock_registers::{LocalRegisterCopy, RegisterLongName, fields::FieldValue};

use crate::reloc::{Relocatable, Relocation};

/// Helper macro to define registers.
#[macro_export]
macro_rules! impl_registers {
    { any, $($(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($reloc:ident) $fields:tt),* $(,)? } => {
        $(
            $crate::__impl_registers!(@munch((C: $crate::__reexports::host1x_client::Client), C) $(#[$inner])* $vis $reg OFFSET($off) RELOCATABLE($reloc) $fields);
        )*
    };
    { $class:ty, $($(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($reloc:ident) $fields:tt),* $(,)? } => {
        $(
            $crate::__impl_registers!(@munch((), $class) $(#[$inner])* $vis $reg OFFSET($off) RELOCATABLE($reloc) $fields);
        )*
    };
}

/// Implementation detail, use [`impl_registers!`] instead.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_registers {
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE(true) [] } => {
        #[allow(nonstandard_style)]
        pub mod $reg {
            use $crate::__reexports::arbitrary_int::u12;
            use $crate::__reexports::host1x_buffer::{ReadableBuffer, WritableBuffer};
            use $crate::__reexports::host1x_client::Client;
            use $crate::reloc::{Relocatable, ReadOnly, ReadWrite, WriteOnly};

            #[derive(Clone, Copy)]
            pub struct Register;
            impl $crate::__reexports::tock_registers::RegisterLongName for Register {}
            impl<$($ty $(: $bound)?),*> $crate::Register<$($class)?> for Register {
                const OFFSET: u12 = u12::new($off);
                type R = Register;
            }

            pub const unsafe fn for_raw_mapping<C: Client>(mapping: u32, offset: u64, shift: u32) -> Relocatable<'static, Register, ReadWrite, C> {
                Relocatable::for_raw_mapping(mapping, offset, shift)
            }

            pub fn read<C: Client>(buf: &impl ReadableBuffer<C>) -> Relocatable<'_, Register, ReadOnly, C> {
                Relocatable::read(buf)
            }

            pub fn write<C: Client>(buf: &mut impl WritableBuffer<C>) -> Relocatable<'_, Register, WriteOnly, C> {
                Relocatable::write(buf)
            }

            pub fn new<C: Client>(buf: &mut (impl ReadableBuffer<C> + WritableBuffer<C>)) -> Relocatable<'_, Register, ReadWrite, C> {
                Relocatable::new(buf)
            }
        }
    };
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE(false) $fields:tt } => {
        #[allow(non_snake_case)]
        $(#[$inner])*
        $vis mod $reg {
            use $crate::__reexports::arbitrary_int::u12;
            use $crate::__reexports::tock_registers::fields::{Field, FieldValue};

            #[derive(Clone, Copy)]
            pub struct Register;
            impl $crate::__reexports::tock_registers::RegisterLongName for Register {}

            /// Create a zero register value.
            pub const fn zero() -> FieldValue<u32, Register> {
                let f = Field::new(u32::MAX, 0);
                f.val(0)
            }

            /// Convert raw bits into register value.
            pub const fn raw(value: u32) -> FieldValue<u32, Register> {
                let f = Field::new(u32::MAX, 0);
                f.val(value)
            }

            // Registers are always 32-bit.
            $crate::__reexports::tock_registers::register_bitmasks!(u32, $reg, Register, $fields);

            impl<$($ty $(: $bound)?),*> $crate::Register<$($class)?> for Register {
                const OFFSET: u12 = u12::new($off);
                type R = Register;
            }
        }
    };
}

/// A register for some device that is behind Host1x interface.
// FIXME: each client has it's own register set, this struct should be generic over
// host1x_client::Client.
pub trait Register<C: Client> {
    /// Register offset in [`u32`] words.
    const OFFSET: u12;
    type R: RegisterLongName;
}

/// A value for [`Register`].
///
/// This is implemented for [`LocalRegisterCopy`] and [`FieldValue`] from [`tock_registers`].
/// Consumers shouldn't need to implement it themselves, to create register definitions
/// use [`tock_registers::register_bitfields!`], then assign offsets to the registers
/// using [`impl_registers!`].
///
/// # Safety
/// Implementors must ensure that depending on [`RegisterValue::HAS_RELOCATION`],
/// [`RegisterValue::reloc`] returns either [`Some<Relocation>`] or [`None`].
pub unsafe trait RegisterValue<C: Client> {
    type R: Register<C>;
    const HAS_RELOCATION: bool;

    fn value(&self) -> u32;
    fn reloc(&self) -> Option<Relocation>;
}
unsafe impl<C: Client, R: RegisterLongName + Register<C>> RegisterValue<C>
    for LocalRegisterCopy<u32, R>
{
    type R = R;
    const HAS_RELOCATION: bool = false;

    fn value(&self) -> u32 {
        self.get()
    }

    fn reloc(&self) -> Option<Relocation> {
        None
    }
}
unsafe impl<C: Client, R: RegisterLongName + Register<C>> RegisterValue<C> for FieldValue<u32, R> {
    type R = R;
    const HAS_RELOCATION: bool = false;

    fn value(&self) -> u32 {
        self.value
    }

    fn reloc(&self) -> Option<Relocation> {
        None
    }
}
unsafe impl<C: Client, R: RegisterLongName + Register<C>, M> RegisterValue<C>
    for Relocatable<'_, R, M, C>
{
    type R = R;
    const HAS_RELOCATION: bool = true;

    fn value(&self) -> u32 {
        // Actual value will be patched-in by whoever performs relocation (typically the kernel-mode driver).
        0
    }

    fn reloc(&self) -> Option<Relocation> {
        Some(Relocation {
            patch_offset: 0,
            mapping: self.mapping(),
            buf_offset: self.buf_offset(),
            shift: self.shift(),
        })
    }
}
