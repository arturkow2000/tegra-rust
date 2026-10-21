//! Buffer memory mapping related types.
//!
//! Access modes of mappings (both CPU and Host1x) are represented by markers
//! from [`host1x_buffer::access`]. This module extends them with the
//! knowledge of how to create such mappings.

use std::{fmt::Debug, marker::PhantomData};

use host1x_buffer::access::{Access, ReadOnly, ReadWrite, WriteOnly};
use rustix::mm::ProtFlags;

use crate::uapi::v2::ioctl::drm_tegra_channel_map_flags;

mod sealed {
    pub trait Sealed {}
}
pub(crate) mod p {
    use std::fmt::Debug;

    use super::{ProtFlags, drm_tegra_channel_map_flags, sealed::Sealed};

    pub trait MappingModePriv: Debug + Sealed {
        /// A hack to effectively implement optional fields.
        ///
        /// Can be either `T`, or [`PhantomData<T>`] which is always zero-size.
        type Handle<T: Debug>: Debug;

        /// Short mode name, used for logging.
        const MODE_NAME: &'static str;

        /// If handle is a real non-phantom type, calls given function to map over handle value.
        /// Otherwise this is a no-op, the given function is not using, allowing the compiler to optimize it out.
        /// Just returns another [`PhantomData<T>`].
        fn map_handle<T: Debug, U: Debug>(
            h: Self::Handle<T>,
            f: impl FnOnce(T) -> U,
        ) -> Self::Handle<U>;

        /// Same as [`MappingModePriv::map_handle`], but takes handle by reference.
        fn map_handle_ref<'a, T: Debug + 'a, U: Debug>(
            h: &'a Self::Handle<T>,
            f: impl FnOnce(&'a T) -> U,
        ) -> Self::Handle<U>;

        /// Same as [`MappingModePriv::map_handle`], but takes handle by mutable reference.
        fn map_handle_mut<'a, T: Debug + 'a, U: Debug>(
            h: &'a mut Self::Handle<T>,
            f: impl FnOnce(&'a mut T) -> U,
        ) -> Self::Handle<U>;

        fn create_cpu_mapping<R: Debug, E>(
            f: impl FnOnce(ProtFlags) -> Result<R, E>,
        ) -> Result<Self::Handle<R>, E>;

        fn destroy_mapping<H: Debug, R>(
            h: Self::Handle<H>,
            default: impl FnOnce() -> R,
            f: impl FnOnce(H) -> R,
        ) -> R;
    }

    pub trait MappedPriv: MappingModePriv {
        fn create_host1x_mapping<R: Debug, E>(
            f: impl FnOnce(drm_tegra_channel_map_flags) -> Result<R, E>,
        ) -> Result<Self::Handle<R>, E>;
    }
}

/// Known memory mapping mode, that we know how to handle at compile time.
///
/// Implemented for [`Unmapped`] and the access markers from [`host1x_buffer::access`].
pub trait MappingMode: p::MappingModePriv {}

/// A real mapping that actually exists ([`MappingMode`] can be [`Unmapped`]).
///
/// Whether the mapping is readable and/or writable is expressed with
/// [`host1x_buffer::access::Readable`] and [`host1x_buffer::access::Writable`].
pub trait Mapped: MappingMode + Access + p::MappedPriv {
    fn get_handle<T: Debug>(h: Self::Handle<T>) -> T;
    fn get_handle_ref<T: Debug>(h: &Self::Handle<T>) -> &T;
    fn get_handle_mut<T: Debug>(h: &mut Self::Handle<T>) -> &mut T;
}

/// No actual mapping.
#[derive(Debug)]
pub struct Unmapped;

impl sealed::Sealed for Unmapped {}
impl p::MappingModePriv for Unmapped {
    type Handle<T: Debug> = PhantomData<T>;
    const MODE_NAME: &'static str = "n/a";

    fn map_handle<T: Debug, U: Debug>(
        _h: Self::Handle<T>,
        _f: impl FnOnce(T) -> U,
    ) -> Self::Handle<U> {
        PhantomData
    }

    fn map_handle_ref<'a, T: Debug + 'a, U: Debug>(
        _h: &'a Self::Handle<T>,
        _f: impl FnOnce(&'a T) -> U,
    ) -> Self::Handle<U> {
        PhantomData
    }

    fn map_handle_mut<'a, T: Debug + 'a, U: Debug>(
        _h: &'a mut Self::Handle<T>,
        _f: impl FnOnce(&'a mut T) -> U,
    ) -> Self::Handle<U> {
        PhantomData
    }

    fn create_cpu_mapping<R: Debug, E>(
        _f: impl FnOnce(ProtFlags) -> Result<R, E>,
    ) -> Result<Self::Handle<R>, E> {
        Ok(PhantomData)
    }

    fn destroy_mapping<H: Debug, R>(
        _h: Self::Handle<H>,
        default: impl FnOnce() -> R,
        _f: impl FnOnce(H) -> R,
    ) -> R {
        default()
    }
}
impl MappingMode for Unmapped {}

/// Implement mapping traits for an access marker, which is always backed by a real mapping.
macro_rules! impl_mapped {
    ($ty:ty, $name:literal, $prot:expr, $map_flags:expr) => {
        impl sealed::Sealed for $ty {}
        impl p::MappingModePriv for $ty {
            type Handle<T: Debug> = T;
            const MODE_NAME: &'static str = $name;

            #[inline]
            fn map_handle<T: Debug, U: Debug>(
                h: Self::Handle<T>,
                f: impl FnOnce(T) -> U,
            ) -> Self::Handle<U> {
                f(h)
            }

            #[inline]
            fn map_handle_ref<'a, T: Debug + 'a, U: Debug>(
                h: &'a Self::Handle<T>,
                f: impl FnOnce(&'a T) -> U,
            ) -> Self::Handle<U> {
                f(h)
            }

            #[inline]
            fn map_handle_mut<'a, T: Debug + 'a, U: Debug>(
                h: &'a mut Self::Handle<T>,
                f: impl FnOnce(&'a mut T) -> U,
            ) -> Self::Handle<U> {
                f(h)
            }

            #[inline]
            fn create_cpu_mapping<R: Debug, E>(
                f: impl FnOnce(ProtFlags) -> Result<R, E>,
            ) -> Result<Self::Handle<R>, E> {
                f($prot)
            }

            #[inline]
            fn destroy_mapping<H: Debug, R>(
                h: Self::Handle<H>,
                _default: impl FnOnce() -> R,
                f: impl FnOnce(H) -> R,
            ) -> R {
                f(h)
            }
        }
        impl p::MappedPriv for $ty {
            #[inline]
            fn create_host1x_mapping<R: Debug, E>(
                f: impl FnOnce(drm_tegra_channel_map_flags) -> Result<R, E>,
            ) -> Result<Self::Handle<R>, E> {
                f($map_flags)
            }
        }
        impl MappingMode for $ty {}
        impl Mapped for $ty {
            #[inline]
            fn get_handle<T: Debug>(h: Self::Handle<T>) -> T {
                h
            }

            #[inline]
            fn get_handle_ref<T: Debug>(h: &Self::Handle<T>) -> &T {
                h
            }

            #[inline]
            fn get_handle_mut<T: Debug>(h: &mut Self::Handle<T>) -> &mut T {
                h
            }
        }
    };
}

impl_mapped!(
    ReadOnly,
    "ro",
    ProtFlags::READ,
    drm_tegra_channel_map_flags::READ
);
impl_mapped!(
    ReadWrite,
    "rw",
    ProtFlags::READ | ProtFlags::WRITE,
    drm_tegra_channel_map_flags::READ | drm_tegra_channel_map_flags::WRITE
);
impl_mapped!(
    WriteOnly,
    "wo",
    ProtFlags::WRITE,
    drm_tegra_channel_map_flags::WRITE
);
