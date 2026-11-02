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

use crate::reloc::{Relocatable, RelocatableRegister, Relocation};

/// Define registers of a Host1x client.
///
/// ```text
/// impl_registers![
///     unsafe crate::Client, // or `unsafe any`, for registers present in every client (Host1x class methods)
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
/// - `RELOCATABLE(ReadOnly)`, `RELOCATABLE(WriteOnly)` and `RELOCATABLE(ReadWrite)` are for
///   registers holding buffer addresses, and state the accesses the GPU makes through them
///   (see [`reloc::RelocatableRegister`]). They have no fields, values are created from
///   branded buffer handles with `read()` (`ReadOnly` registers only), `write()`
///   (`WriteOnly` registers only) or `new()`, or with the unsafe `for_raw_mapping()`.
///   Relocatable registers are always unsafe, so `UNSAFE(false)` is rejected for them.
///
/// # Safety
/// The macro implements the unsafe [`Register`] trait, so the invocation must start with
/// `unsafe`, and the caller must uphold its requirements for every register: offsets must be
/// correct, and registers whose writes can break safety guarantees must not be declared
/// plain.
///
/// # Examples
///
/// ```
/// # use host1x_reg::{RegisterValue, impl_registers};
/// # pub struct Client;
/// # unsafe impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     unsafe crate::Client,
///     pub CTRL OFFSET(0x01) RELOCATABLE(false) [ ENABLE OFFSET(0) NUMBITS(1) [] ],
///     /// # Safety
///     /// Writing must not do anything bad.
///     pub DANGER OFFSET(0x02) RELOCATABLE(false) UNSAFE(true) [ VALUE OFFSET(0) NUMBITS(8) [] ],
///     pub ADDR OFFSET(0x03) RELOCATABLE(WriteOnly) [],
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
/// # unsafe impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     unsafe crate::Client,
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
/// # unsafe impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     unsafe crate::Client,
///     pub ADDR OFFSET(0x03) RELOCATABLE(WriteOnly) [],
/// ];
///
/// fn write<V: RegisterValue<Client>>(_: V) {}
/// fn main() {
///     write(tock_registers::LocalRegisterCopy::<u32, ADDR::Register>::new(0x8000_0000));
/// }
/// ```
///
/// Nor can a register the GPU writes through be given a read-only relocation:
///
/// ```compile_fail,E0271
/// # use host1x_buffer::ReadableBuffer;
/// # use host1x_reg::impl_registers;
/// # pub struct Client;
/// # unsafe impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     unsafe crate::Client,
///     pub ADDR OFFSET(0x03) RELOCATABLE(WriteOnly) [],
/// ];
///
/// fn read<'id>(buf: &impl ReadableBuffer<'id, Client>) {
///     let _ = ADDR::read(buf);
/// }
/// # fn main() {}
/// ```
///
/// Relocatable registers can't be declared safe:
///
/// ```compile_fail
/// # use host1x_reg::impl_registers;
/// # pub struct Client;
/// # unsafe impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     unsafe crate::Client,
///     pub ADDR OFFSET(0x03) RELOCATABLE(WriteOnly) UNSAFE(false) [],
/// ];
/// # fn main() {}
/// ```
///
/// Nor can registers be declared without `unsafe`:
///
/// ```compile_fail
/// # use host1x_reg::impl_registers;
/// # pub struct Client;
/// # unsafe impl host1x_client::Client for Client {
/// #     const CLASS: arbitrary_int::u10 = arbitrary_int::u10::new(0x51);
/// # }
/// impl_registers![
///     crate::Client,
///     pub CTRL OFFSET(0x01) RELOCATABLE(false) [ ENABLE OFFSET(0) NUMBITS(1) [] ],
/// ];
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! impl_registers {
    { unsafe any, $($(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($reloc:ident) $(UNSAFE($unsafe:ident))? [$($fields:tt)*]),* $(,)? } => {
        $(
            $crate::__impl_registers!(@munch((C: $crate::__reexports::host1x_client::Client), C) $(#[$inner])* $vis $reg OFFSET($off) RELOCATABLE($reloc) UNSAFE($($unsafe)?) [$($fields)*]);
        )*
    };
    { unsafe $class:ty, $($(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($reloc:ident) $(UNSAFE($unsafe:ident))? [$($fields:tt)*]),* $(,)? } => {
        $(
            $crate::__impl_registers!(@munch((), $class) $(#[$inner])* $vis $reg OFFSET($off) RELOCATABLE($reloc) UNSAFE($($unsafe)?) [$($fields)*]);
        )*
    };
}

/// Implementation detail, use [`impl_registers!`] instead.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_registers {
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
            unsafe impl<$($ty $(: $bound)?),*> $crate::Register<$($class)?> for Register {
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

            unsafe impl<$($ty $(: $bound)?),*> $crate::Register<$($class)?> for Register {
                const OFFSET: u12 = u12::new($off);
            }
        }
    };
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE(true) $($rest:tt)* } => {
        ::core::compile_error!(::core::concat!(::core::stringify!($reg), ": give the GPU access instead: RELOCATABLE(ReadOnly), RELOCATABLE(WriteOnly) or RELOCATABLE(ReadWrite)"));
    };
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($access:ident) UNSAFE(false) $fields:tt } => {
        ::core::compile_error!(::core::concat!(::core::stringify!($reg), ": relocatable registers are always unsafe, they can't be UNSAFE(false)"));
    };
    { @munch(($($ty:ident $(: $bound:path)?),*)$(, $class:ty)?) $(#[$inner:meta])* $vis:vis $reg:ident OFFSET($off:literal) RELOCATABLE($access:ident) UNSAFE($(true)?) [] } => {
        #[allow(nonstandard_style)]
        $(#[$inner])*
        $vis mod $reg {
            use $crate::__reexports::arbitrary_int::u12;
            use $crate::__reexports::host1x_buffer::{ReadableBuffer, WritableBuffer};
            use $crate::__reexports::host1x_client::Client;
            use $crate::reloc::{Relocatable, RelocatableRegister, ReadOnly, ReadWrite, WriteOnly};

            #[derive(Clone, Copy)]
            pub struct Register;
            // Deliberately does not implement RegisterLongName so tock_registers can't be used
            // to bypass constructors.
            unsafe impl<$($ty $(: $bound)?),*> $crate::Register<$($class)?> for Register {
                const OFFSET: u12 = u12::new($off);
            }
            unsafe impl<$($ty $(: $bound)?),*> RelocatableRegister<$($class)?> for Register {
                type Access = $crate::reloc::$access;
            }

            /// # Safety
            /// See [`Relocatable::for_raw_mapping`].
            pub const unsafe fn for_raw_mapping<'id, C: Client>(mapping: u32, offset: u64, shift: u32) -> Relocatable<'id, Register, ReadWrite, C> {
                unsafe { Relocatable::for_raw_mapping(mapping, offset, shift) }
            }

            /// Available if the GPU only reads through the register.
            pub fn read<'id, C: Client>(buf: &impl ReadableBuffer<'id, C>) -> Relocatable<'id, Register, ReadOnly, C>
            where
                Register: RelocatableRegister<C, Access = ReadOnly>,
            {
                Relocatable::read(buf)
            }

            /// Available if the GPU only writes through the register.
            pub fn write<'id, C: Client>(buf: &mut impl WritableBuffer<'id, C>) -> Relocatable<'id, Register, WriteOnly, C>
            where
                Register: RelocatableRegister<C, Access = WriteOnly>,
            {
                Relocatable::write(buf)
            }

            pub fn new<'id, C: Client>(buf: &mut (impl ReadableBuffer<'id, C> + WritableBuffer<'id, C>)) -> Relocatable<'id, Register, ReadWrite, C>
            where
                Register: RelocatableRegister<C>,
            {
                Relocatable::new(buf)
            }
        }
    };
}

/// A register for some device that is behind Host1x interface.
///
/// Define registers with [`impl_registers!`] instead of implementing this by hand.
///
/// # Safety
/// Commands write values of registers, and tasks trust their types: any [`RegisterValue`]
/// with `R = Self` that safe code can create must be safe to write, unless it's only unsafe
/// in combination with other writes that tasks must already get right (such as buffer
/// geometry matching the relocated buffer). In particular, if
/// `Self` implements [`RegisterLongName`], any value can be created through
/// [`FieldValue`] or [`LocalRegisterCopy`], so writing any value to the register must be
/// safe. `OFFSET` must be the offset of the register in client `C`.
pub unsafe trait Register<C: Client> {
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
unsafe impl<C: Client, R: RelocatableRegister<C>, M> RegisterValue<C> for Relocatable<'_, R, M, C> {
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
