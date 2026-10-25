//! Utilities for working with sequences of commands.

use core::{iter::FusedIterator, mem::MaybeUninit, num::NonZeroUsize, ops::Deref, slice};

use host1x_client::Client;
use host1x_reg::reloc::Relocation;
use snafu::{Snafu, ensure};
use zerocopy::little_endian::U32 as U32LE;

mod p {
    pub trait TargetSlicePriv<'a> {
        fn as_uninit_u32_slice_mut(&mut self) -> &mut [super::MaybeUninit<u32>];
        unsafe fn assume_init_u32_slice(self, n: usize) -> &'a [u32];
    }
}

/// Slice too small error.
#[derive(Debug, Snafu, Clone, Copy, PartialEq, Eq)]
#[snafu(display("buffer is too small, need {needed} more 32-bit words of capacity"))]
pub struct BufferTooSmallError {
    pub needed: NonZeroUsize,
}

/// A slice for storing generated command stream.
///
/// This trait unifies `&mut [u32]` and `&mut [MaybeUninit<u32>]`.
pub trait TargetSlice<'a>: p::TargetSlicePriv<'a> {}
impl<'a> p::TargetSlicePriv<'a> for &'a mut [u32] {
    fn as_uninit_u32_slice_mut(&mut self) -> &mut [MaybeUninit<u32>] {
        // Safety: MaybeUninit<T> is guaranteed to have same layout as T
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr() as *mut MaybeUninit<u32>, self.len()) }
    }

    unsafe fn assume_init_u32_slice(self, n: usize) -> &'a [u32] {
        debug_assert!(n <= self.len());
        // Safety: caller must ensure `n` <= `self.len()`
        unsafe { self.get_unchecked(..n) }
    }
}
impl<'a> TargetSlice<'a> for &'a mut [u32] {}
impl<'a> p::TargetSlicePriv<'a> for &'a mut [MaybeUninit<u32>] {
    fn as_uninit_u32_slice_mut(&mut self) -> &mut [MaybeUninit<u32>] {
        self
    }

    unsafe fn assume_init_u32_slice(self, n: usize) -> &'a [u32] {
        debug_assert!(n <= self.len());
        // TODO: use array_assume_init when https://github.com/rust-lang/rust/issues/96097 lands
        // Safety:
        // - MaybeUninit<T> is guaranteed to have same layout as T
        // - caller must ensure `self` has been initialized up to `n` elements.
        // - caller must ensure `n` <= `self.len()`
        unsafe { slice::from_raw_parts(self.as_ptr() as *const u32, n) }
    }
}
impl<'a> TargetSlice<'a> for &'a mut [MaybeUninit<u32>] {}

macro_rules! count_words {
    ($($C:ident),+) => {
        0 $(+ $C::N_WORDS)+
    };
}
macro_rules! count_relocations {
    ($($C:ident),+) => {
        0 $(+ $C::N_RELOCS)+
    }
}
macro_rules! impl_command_sequence {
    ($(($C:ident, $idx:tt)),+) => {
        unsafe impl<CL: Client, $($C: CommandSequence<CL>),+> CommandSequence<CL> for ($($C),+,) {
            const N_WORDS: usize = count_words!($($C),+);
            const N_RELOCS: usize = count_relocations!($($C),+);

            fn try_write_to_slice<'target>(&self, mut dest: impl TargetSlice<'target>) -> Result<ValidRawCommandArray<'target>, BufferTooSmallError> {
                let uninit = dest.as_uninit_u32_slice_mut();
                ensure!(uninit.len() >= Self::N_WORDS, BufferTooSmallSnafu { needed: NonZeroUsize::new(Self::N_WORDS - uninit.len()).unwrap() });

                let mut off = 0;
                $(
                    let mut words = self.$idx.words();
                    let words_len = words.len();
                    debug_assert_eq!($C::N_WORDS, words_len);
                    for i in 0..words.len() {
                        // Safety:
                        // - Command trait unsafe, implementors must ensure it Command::words returns proper number of elements.
                        // - dest has been checked for correct length above
                        unsafe { *uninit.get_unchecked_mut(off + i) = MaybeUninit::new(words.next().unwrap_unchecked().get().to_le()); }
                    }
                    off += words_len;
                )+

                debug_assert_eq!(off, Self::N_WORDS);

                // Safety: entire array has been initialized above.
                Ok(unsafe { ValidRawCommandArray::assume_valid(dest.assume_init_u32_slice(Self::N_WORDS)) })
            }

            fn words(&self) -> impl ExactSizeIterator<Item = U32LE> {
                // All the iterators we are chaining already are ExactSizeIterator
                // and we already know the exact number of elements each iterator will return.
                struct ExactSize<I> {
                    it: I,
                    size: usize
                }
                impl<I: Iterator> Iterator for ExactSize<I> {
                    type Item = I::Item;

                    fn next(&mut self) -> Option<Self::Item> {
                        let item = self.it.next()?;
                        self.size -= 1;
                        Some(item)
                    }

                    fn size_hint(&self) -> (usize, Option<usize>) {
                        (self.size, Some(self.size))
                    }
                }
                impl<I: Iterator> ExactSizeIterator for ExactSize<I> {
                    fn len(&self) -> usize {
                        self.size
                    }
                }
                impl<I: FusedIterator> FusedIterator for ExactSize<I> {}

                ExactSize {
                    it: [].into_iter()
                        $(
                            .chain({
                                let words = self.$idx.words();
                                debug_assert_eq!($C::N_WORDS, words.len());
                                words
                            })
                        )+
                    ,
                    size: <Self as CommandSequence<CL>>::N_WORDS,
                }
            }

            fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation> {
                // All the iterators we are chaining already are ExactSizeIterator
                // and we already know the exact number of elements each iterator will return.
                struct ExactSize<I> {
                    it: I,
                    size: usize
                }
                impl<I: Iterator> Iterator for ExactSize<I> {
                    type Item = I::Item;

                    fn next(&mut self) -> Option<Self::Item> {
                        let item = self.it.next()?;
                        self.size -= 1;
                        Some(item)
                    }

                    fn size_hint(&self) -> (usize, Option<usize>) {
                        (self.size, Some(self.size))
                    }
                }
                impl<I: Iterator> ExactSizeIterator for ExactSize<I> {
                    fn len(&self) -> usize {
                        self.size
                    }
                }
                impl<I: FusedIterator> FusedIterator for ExactSize<I> {}

                let mut off = 0;
                ExactSize {
                    it: [].into_iter()
                        $(
                            .chain({
                                let relocs = self.$idx.relocs();
                                debug_assert_eq!($C::N_RELOCS, relocs.len());
                                let relocs = relocs.map(move |mut r| {
                                    r.patch_offset += off as u32;
                                    r
                                });
                                #[allow(unused)]
                                { off += $C::N_WORDS; }
                                relocs
                            })
                        )+
                    ,
                    size: <Self as CommandSequence<CL>>::N_RELOCS,
                }
            }
        }
    };
}

/// A sequence of one or more valid Host1x commands.
///
/// # Safety
/// - [`CommandSequence::words`] must return exactly [`CommandSequence::N_WORDS`] elements.
/// - [`CommandSequence::relocs`] must return exactly [`CommandSequence::N_RELOCS`] elements.
/// - [`CommandSequence::try_write_to_slice`] must write exactly the same words that [`CommandSequence::words`] returns.
pub unsafe trait CommandSequence<C: Client> {
    /// Total number of words for entire command stream.
    const N_WORDS: usize;

    /// Total number of relocations in entire command stream.
    const N_RELOCS: usize;

    fn try_write_to_slice<'target>(
        &self,
        mut dest: impl TargetSlice<'target>,
    ) -> Result<ValidRawCommandArray<'target>, BufferTooSmallError> {
        let uninit = dest.as_uninit_u32_slice_mut();
        ensure!(
            uninit.len() >= Self::N_WORDS,
            BufferTooSmallSnafu {
                needed: NonZeroUsize::new(Self::N_WORDS - uninit.len()).unwrap()
            }
        );

        let mut words = self.words();
        let words_len = words.len();
        debug_assert_eq!(Self::N_WORDS, words_len);
        for i in 0..words.len() {
            // Safety:
            // - Command trait unsafe, implementors must ensure it Command::words returns proper number of elements.
            // - dest has been checked for correct length above
            unsafe {
                *uninit.get_unchecked_mut(i) =
                    MaybeUninit::new(words.next().unwrap_unchecked().get().to_le());
            }
        }

        // Safety: entire array has been initialized above.
        Ok(
            unsafe {
                ValidRawCommandArray::assume_valid(dest.assume_init_u32_slice(Self::N_WORDS))
            },
        )
    }

    fn write_to_slice<'target>(
        &self,
        dest: impl TargetSlice<'target>,
    ) -> ValidRawCommandArray<'target> {
        self.try_write_to_slice(dest)
            .expect("target slice is too small")
    }

    /// Return an iterator of raw 32-bit words as understood by the HOST1X interface.
    fn words(&self) -> impl ExactSizeIterator<Item = U32LE>;

    /// High-level description of relocations to be performed prior to command submission.
    fn relocs(&self) -> impl ExactSizeIterator<Item = Relocation>;
}
// This is extremely slow to compile. With impl's for up to 8 commands (which is
// the minimum required to clear buffer to a single color using GR2D) it takes
// around 10 seconds to compile host1x-core. With default impl for 64 commands
// it takes more than a minute for every single, even the smallest change to
// host1x-core or its dependencies.
//
// The machine used for testing 6-core AMD 4650G with 48 GiB of RAM.
//
// The main offenders are (-Ztime-passes)
// time:  42.209; rss:  278MB ->  568MB ( +290MB)  coherence_checking
// time:  45.170; rss:  278MB ->  630MB ( +351MB)  type_check_crate
// time:   6.768; rss:  733MB ->  742MB (   +9MB)  module_lints
// time:   6.768; rss:  733MB ->  742MB (   +9MB)  lint_checking
// time:   6.909; rss:  733MB ->  742MB (   +9MB)  misc_checking_3
// time:  55.817; rss:   28MB ->  104MB (  +75MB)  total
//
// Debug vs release nor -Zthread don't seem to make any difference.
// Luckily, this issue only happens with host1x-core, modifying and rebuilding
// any of libraries/binaries which have dependency on host1x-core is fast as
// usually and takes fractions of second.
//
// For development of host1x-core or one of its dependencies, you may just comment-out
// most of impls to get orders of magnitude faster compilation times.
//
// There is also some issue with rust-analyzer consuming large amounts of memory
// (quickly filling up 48 GiB of RAM + swap) but it happens only when this file
// is open in vscode, so keep that in mind, especially if you are running on Linux
// and don't have userspace OOM killer. This issue can also be mitigated by
// commenting out unneeded impl_command_sequence!() calls.
#[rustfmt::skip] impl_command_sequence!((C0, 0));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13));
#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56), (C57, 57));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56), (C57, 57), (C58, 58));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56), (C57, 57), (C58, 58), (C59, 59));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56), (C57, 57), (C58, 58), (C59, 59), (C60, 60));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56), (C57, 57), (C58, 58), (C59, 59), (C60, 60), (C61, 61));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56), (C57, 57), (C58, 58), (C59, 59), (C60, 60), (C61, 61), (C62, 62));
//#[rustfmt::skip] impl_command_sequence!((C0, 0), (C1, 1), (C2, 2), (C3, 3), (C4, 4), (C5, 5), (C6, 6), (C7, 7), (C8, 8), (C9, 9), (C10, 10), (C11, 11), (C12, 12), (C13, 13), (C14, 14), (C15, 15), (C16, 16), (C17, 17), (C18, 18), (C19, 19), (C20, 20), (C21, 21), (C22, 22), (C23, 23), (C24, 24), (C25, 25), (C26, 26), (C27, 27), (C28, 28), (C29, 29), (C30, 30), (C31, 31), (C32, 32), (C33, 33), (C34, 34), (C35, 35), (C36, 36), (C37, 37), (C38, 38), (C39, 39), (C40, 40), (C41, 41), (C42, 42), (C43, 43), (C44, 44), (C45, 45), (C46, 46), (C47, 47), (C48, 48), (C49, 49), (C50, 50), (C51, 51), (C52, 52), (C53, 53), (C54, 54), (C55, 55), (C56, 56), (C57, 57), (C58, 58), (C59, 59), (C60, 60), (C61, 61), (C62, 62), (C63, 63));

/// An array of raw Host1x commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidRawCommandArray<'a>(&'a [u32]);
impl<'a> ValidRawCommandArray<'a> {
    /// Assume given [`u32`] slice is a valid command array.
    ///
    /// # Safety
    /// Given slice must be valid array of Host1x commands. APIs that take [`ValidRawCommandArray`]
    /// as argument expect these assumption to hold and may behave erratically otherwise.
    pub const unsafe fn assume_valid(arr: &'a [u32]) -> Self {
        Self(arr)
    }

    pub const fn into_inner(self) -> &'a [u32] {
        self.0
    }

    /// Return raw, binary (little endian!) encoding of Host1x commands.
    pub const fn get(&self) -> &[u32] {
        self.0
    }
}
impl Deref for ValidRawCommandArray<'_> {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use gr2d_reg::{Client as Gr2d, SBFMT0};
    use std::{mem::MaybeUninit, num::NonZero};

    use crate::{
        cmd::{SetClass, WriteNoIncr},
        cmdseq::ValidRawCommandArray,
    };

    use super::{BufferTooSmallError, CommandSequence};

    #[test]
    fn test_cmd_seq() {
        let mut dest = const { [MaybeUninit::uninit(); 3] };
        assert_eq!(
            SetClass::<_, _, Gr2d, _>::new()
                .try_write_to_slice(&mut dest[..])
                .map(ValidRawCommandArray::into_inner),
            Ok(&[0x1440][..])
        );

        let mut dest = const { [0; 1] };
        assert_eq!(
            (
                SetClass::<_, _, Gr2d, _>::new(),
                WriteNoIncr::new([SBFMT0::DIFMT::B5G6R5 + SBFMT0::SIFMT::R8G8B8A8]),
            )
                .try_write_to_slice(&mut dest[..])
                .map(ValidRawCommandArray::into_inner),
            Err(BufferTooSmallError {
                needed: NonZero::new(2).unwrap()
            })
        );

        let mut dest = const { [MaybeUninit::uninit(); 3] };
        assert_eq!(
            (
                SetClass::<_, _, Gr2d, _>::new(),
                WriteNoIncr::new([SBFMT0::DIFMT::B5G6R5 + SBFMT0::SIFMT::R8G8B8A8]),
            )
                .try_write_to_slice(&mut dest[..])
                .map(ValidRawCommandArray::into_inner),
            Ok(&[0x1440, 0x201c0001, 0x080e,][..])
        );
    }
}
