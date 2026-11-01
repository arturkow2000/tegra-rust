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

/// Define registers of a Host1x client.
///
/// ```text
/// impl_registers![
///     crate::Client, // or `any`, for registers present in every client (Host1x class methods)
///     /// Documentation of the register.
///     pub NAME OFFSET(0x2b) RELOCATABLE(false) UNSAFE(false) [ fields ],
///     ...
/// ];
/// ```
///
/// Each register becomes a module named after it. `UNSAFE` is optional and defaults to
/// `false`. What the module contains depends on the options:
///
/// - Plain registers (`RELOCATABLE(false)`, `UNSAFE(false)`) get [`tock_registers`] fields
///   and `zero()` / `raw()`. Their values can be created freely and written directly.
/// - `UNSAFE(true)` is for registers whose writes can break safety guarantees, such as
///   syncpoint increments. Fields are defined on the module's `Fields` type, which isn't a
///   [`Register`], so values built from them have to be passed to the unsafe `new()` to get
///   a value that can be written. Document the safety requirements of writing the register
///   on the register itself.
/// - `RELOCATABLE(true)` is for registers holding buffer addresses. They have no fields,
///   values are created from branded buffer handles with `read()`, `write()` or `new()`,
///   or with the unsafe `for_raw_mapping()`. Relocatable registers are always unsafe, so
///   `RELOCATABLE(true) UNSAFE(false)` is rejected.
///
/// # Examples
///
/// ```
/// # use host1x_reg::{RegisterValue, impl_registers};
/// # pub struct Client;
/// # impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     crate::Client,
///     pub CTRL OFFSET(0x01) RELOCATABLE(false) [ ENABLE OFFSET(0) NUMBITS(1) [] ],
///     /// # Safety
///     /// Writing must not do anything bad.
///     pub DANGER OFFSET(0x02) RELOCATABLE(false) UNSAFE(true) [ VALUE OFFSET(0) NUMBITS(8) [] ],
///     pub ADDR OFFSET(0x03) RELOCATABLE(true) [],
/// ];
///
/// fn write<V: RegisterValue<Client>>(_: V) {}
/// fn main() {
///     write(CTRL::ENABLE.val(1));
///     // Safety: doctest, nothing is written to the GPU.
///     write(unsafe { DANGER::new(DANGER::VALUE.val(1)) });
/// }
/// ```
///
/// Values of unsafe registers can't be written without `new()`:
///
/// ```compile_fail,E0277
/// # use host1x_reg::{RegisterValue, impl_registers};
/// # pub struct Client;
/// # impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     crate::Client,
///     pub DANGER OFFSET(0x02) RELOCATABLE(false) UNSAFE(true) [ VALUE OFFSET(0) NUMBITS(8) [] ],
/// ];
///
/// fn write<V: RegisterValue<Client>>(_: V) {}
/// fn main() {
///     write(DANGER::VALUE.val(1));
/// }
/// ```
///
/// Nor can relocatable registers be written with a raw address:
///
/// ```compile_fail,E0277
/// # use host1x_reg::{RegisterValue, impl_registers};
/// # pub struct Client;
/// # impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     crate::Client,
///     pub ADDR OFFSET(0x03) RELOCATABLE(true) [],
/// ];
///
/// fn write<V: RegisterValue<Client>>(_: V) {}
/// fn main() {
///     write(tock_registers::LocalRegisterCopy::<u32, ADDR::Register>::new(0x8000_0000));
/// }
/// ```
///
/// Relocatable registers can't be declared safe:
///
/// ```compile_fail
/// # use host1x_reg::impl_registers;
/// # pub struct Client;
/// # impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     crate::Client,
///     pub ADDR OFFSET(0x03) RELOCATABLE(true) UNSAFE(false) [],
/// ];
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! impl_registers {
    { any, $($(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($reloc:ident) $(UNSAFE($unsafe:ident))? [$($fields:tt)*]),* $(,)? } => {
        $(
            $crate::__impl_registers!(@munch((C: $crate::__reexports::host1x_client::Client), C) $(#[$inner])* $vis $reg OFFSET($off) RELOCATABLE($reloc) UNSAFE($($unsafe)?) [$($fields)*]);
        )*
    };
    { $class:ty, $($(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($reloc:ident) $(UNSAFE($unsafe:ident))? [$($fields:tt)*]),* $(,)? } => {
        $(
            $crate::__impl_registers!(@munch((), $class) $(#[$inner])* $vis $reg OFFSET($off) RELOCATABLE($reloc) UNSAFE($($unsafe)?) [$($fields)*]);
        )*
    };
}

/// Implementation detail, use [`impl_registers!`] instead.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_registers {
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE(true) UNSAFE($(true)?) [] } => {
        #[allow(nonstandard_style)]
        $(#[$inner])*
        $vis mod $reg {
            use $crate::__reexports::arbitrary_int::u12;
            use $crate::__reexports::host1x_buffer::{ReadableBuffer, WritableBuffer};
            use $crate::__reexports::host1x_client::Client;
            use $crate::reloc::{Relocatable, ReadOnly, ReadWrite, WriteOnly};

            #[derive(Clone, Copy)]
            pub struct Register;
            // Deliberately does not implement RegisterLongName so tock_registers can't be used
            // to bypass constructors.
            impl<$($ty $(: $bound)?),*> $crate::Register<$($class)?> for Register {
                const OFFSET: u12 = u12::new($off);
            }

            /// # Safety
            /// See [`Relocatable::for_raw_mapping`].
            pub const unsafe fn for_raw_mapping<'id, C: Client>(mapping: u32, offset: u64, shift: u32) -> Relocatable<'id, Register, ReadWrite, C> {
                unsafe { Relocatable::for_raw_mapping(mapping, offset, shift) }
            }

            pub fn read<'id, C: Client>(buf: &impl ReadableBuffer<'id, C>) -> Relocatable<'id, Register, ReadOnly, C> {
                Relocatable::read(buf)
            }

            pub fn write<'id, C: Client>(buf: &mut impl WritableBuffer<'id, C>) -> Relocatable<'id, Register, WriteOnly, C> {
                Relocatable::write(buf)
            }

            pub fn new<'id, C: Client>(buf: &mut (impl ReadableBuffer<'id, C> + WritableBuffer<'id, C>)) -> Relocatable<'id, Register, ReadWrite, C> {
                Relocatable::new(buf)
            }
        }
    };
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE(true) UNSAFE(false) $fields:tt } => {
        ::core::compile_error!(::core::concat!(::core::stringify!($reg), ": RELOCATABLE(true) implies UNSAFE(true)"));
    };
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE(false) UNSAFE(true) $fields:tt } => {
        #[allow(non_snake_case)]
        $(#[$inner])*
        $vis mod $reg {
            use $crate::__reexports::arbitrary_int::u12;
            use $crate::__reexports::tock_registers::{RegisterLongName, fields::{Field, FieldValue}};

            /// The register.
            ///
            /// Deliberately not [`RegisterLongName`], so values of it can only be created with
            /// the unsafe [`new`].
            #[derive(Clone, Copy)]
            pub struct Register;
            impl<$($ty $(: $bound)?),*> $crate::Register<$($class)?> for Register {
                const OFFSET: u12 = u12::new($off);
            }

            /// Fields of the register.
            ///
            /// Deliberately not a register, values built from fields have to be passed
            /// to the unsafe [`new`] before writing them.
            #[derive(Clone, Copy)]
            pub struct Fields;
            impl RegisterLongName for Fields {}

            /// Create zero fields value, see [`new`].
            pub const fn zero() -> FieldValue<u32, Fields> {
                let f = Field::new(u32::MAX, 0);
                f.val(0)
            }

            /// Convert raw bits into fields value, see [`new`].
            pub const fn raw(value: u32) -> FieldValue<u32, Fields> {
                let f = Field::new(u32::MAX, 0);
                f.val(value)
            }

            // Registers are always 32-bit.
            $crate::__reexports::tock_registers::register_bitmasks!(u32, $reg, Fields, $fields);

            /// A value of [`Register`], see [`new`].
            #[derive(Debug, Clone, Copy)]
            pub struct Value(u32);

            /// Create a value of the register.
            ///
            /// # Safety
            /// The register is unsafe to write. The caller must uphold the safety requirements
            /// from the register's documentation for every write of the value.
            pub const unsafe fn new(value: FieldValue<u32, Fields>) -> Value {
                Value(value.value)
            }

            // Safety: no relocation.
            unsafe impl<$($ty $(: $bound)?),*> $crate::RegisterValue<$($class)?> for Value {
                type R = Register;
                const HAS_RELOCATION: bool = false;

                fn value(&self) -> u32 {
                    self.0
                }

                fn reloc(&self) -> ::core::option::Option<$crate::reloc::Relocation> {
                    ::core::option::Option::None
                }
            }
        }
    };
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE(false) UNSAFE($(false)?) $fields:tt } => {
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
}

/// A value for [`Register`].
///
/// For plain registers this is implemented for [`LocalRegisterCopy`] and [`FieldValue`] from
/// [`tock_registers`]. Registers that are unsafe to write have their own value types instead
/// ([`Relocatable`] and `Value` of `UNSAFE(true)` registers). Consumers shouldn't need to
/// implement it themselves, define registers with [`impl_registers!`].
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
unsafe impl<C: Client, R: Register<C>, M> RegisterValue<C> for Relocatable<'_, R, M, C> {
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
