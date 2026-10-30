//! Building sequences of GPU tasks.
//!
//! A [`Job`] takes ownership of the buffers used by the GPU and runs a
//! [`Pipeline`] over [`Handle`]s for them. Handles are branded with a unique
//! lifetime `'id` (created by [`guard!`]) so they can only be used with
//! tasks of the very job that owns the buffers. Each stage of the pipeline
//! consumes handles and gives them back as its output, passing them on to the
//! next stage. Buffers are given back once the GPU is done.
//!
//! The whole command stream is a single statically-typed [`CommandSequence`],
//! so its size is known at compile time.
//!
//! Jobs are most conveniently created with [`job!`](crate::job!), which takes care of
//! creating the brand:
//!
//! ```ignore
//! let job = job!(mapping, (
//!     |buf| gr2d::clear(buf),
//!     |buf| gr2d::fill_rect(buf, color, rect),
//! ));
//! let mapping = queue.submit(&mut fence, job).await?.await?;
//! ```

use core::{fmt, marker::PhantomData};

use generativity::Id;
pub use generativity::{Guard, guard};
use host1x_buffer::{
    Buffer, Layout, MappedBuffer, PixelLayout, ReadableBuffer, WritableBuffer,
    access::{Access, Readable, Writable},
};
use host1x_client::Client;

use crate::{cmdseq::CommandSequence, queue::Channel, task::Task};

/// Unique brand of a [`Job`].
///
/// It can't be created outside of [`Job::new`], so having a reference to it
/// proves that buffers are being imported into the job.
pub struct Brand<'id>(Id<'id>);
impl fmt::Debug for Brand<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Brand").field(&self.0).finish()
    }
}

/// GPU-side handle to a buffer owned by a [`Job`].
///
/// The handle carries only buffer description and mapping ID, CPU access to the buffer
/// is not possible. Handles are neither [`Copy`] nor [`Clone`]: a task that takes
/// a handle by value has exclusive access to the buffer and returns the handle
/// when done. Read-only access can be shared by passing a reference.
pub struct Handle<'id, C: Client, L, A: Access> {
    mapping: u32,
    desc: L,
    _brand: Id<'id>,
    _phantom: PhantomData<(fn() -> C, A)>,
}
impl<'id, C: Client, L, A: Access> Handle<'id, C, L, A> {
    /// Create a new handle.
    ///
    /// # Safety
    /// `mapping` must be a valid mapping ID of a buffer owned by the job that
    /// issued `brand`, mapped into address space of `C`. The mapping must allow
    /// access requested by `A`, and `desc` must describe the buffer accurately.
    pub unsafe fn new(brand: &Brand<'id>, mapping: u32, desc: L) -> Self {
        Self {
            mapping,
            desc,
            _brand: brand.0,
            _phantom: PhantomData,
        }
    }
}
impl<C: Client, L: fmt::Debug, A: Access> fmt::Debug for Handle<'_, C, L, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("mapping", &self.mapping)
            .field("desc", &self.desc)
            .finish()
    }
}
impl<C: Client, L: Layout, A: Access> Layout for Handle<'_, C, L, A> {
    fn size(&self) -> u32 {
        self.desc.size()
    }
}
impl<C: Client, L: PixelLayout, A: Access> PixelLayout for Handle<'_, C, L, A> {
    type Format = L::Format;

    fn width(&self) -> u32 {
        self.desc.width()
    }

    fn height(&self) -> u32 {
        self.desc.height()
    }

    fn stride(&self) -> u32 {
        self.desc.stride()
    }
}
impl<C: Client, L: Layout + Copy, A: Access> Buffer for Handle<'_, C, L, A> {
    type Desc = L;

    fn desc(&self) -> L {
        self.desc
    }
}
// Safety: guaranteed by `Handle::new` contract, layout is taken from `desc`.
unsafe impl<'id, C: Client, L: Layout + Copy, A: Access> MappedBuffer<'id, C>
    for Handle<'id, C, L, A>
{
    fn mapping_id(&self) -> u32 {
        self.mapping
    }
}
// Safety: `Handle::new` requires the mapping to allow access requested by `A`.
unsafe impl<'id, C: Client, L: Layout + Copy, A: Readable> ReadableBuffer<'id, C>
    for Handle<'id, C, L, A>
{
}
// Safety: `Handle::new` requires the mapping to allow access requested by `A`.
unsafe impl<'id, C: Client, L: Layout + Copy, A: Writable> WritableBuffer<'id, C>
    for Handle<'id, C, L, A>
{
}

/// A buffer mapped into address space of a Host1x client, which can be moved
/// into a [`Job`].
///
/// Implemented by backends for their buffer mappings. Every [`Mapping`] is
/// [`Import`], handles to it carry the buffer description ([`Buffer::Desc`]).
///
/// # Safety
/// - [`Mapping::mapping_id`] must return ID of a mapping owned by `self`, which
///   stays valid for as long as `self` is alive and allows access requested by
///   [`Mapping::Access`].
/// - [`Buffer::desc`] must describe the buffer accurately.
/// - While `self` is owned by a job it must not be possible to access buffer
///   memory through any other path.
pub unsafe trait Mapping: Buffer {
    type Client: Client;
    type Access: Access;

    /// Opaque identifier of the mapping. Used for relocations.
    fn mapping_id(&self) -> u32;
}

/// Buffers that can be moved into a [`Job`].
///
/// Implemented for every [`Mapping`], and for tuples of such buffers.
///
/// `()` is deliberately not [`Import`], so a job always owns at least one buffer.
///
/// This trait is sealed, backends implement [`Mapping`] instead.
pub trait Import: sealed::SealedImport {
    type Handles<'id>;

    /// Create handles for the buffers.
    fn import<'id>(&self, brand: &Brand<'id>) -> Self::Handles<'id>;
}

impl<B: Mapping> sealed::SealedImport for B {}
impl<B: Mapping> Import for B {
    type Handles<'id> = Handle<'id, B::Client, B::Desc, B::Access>;

    fn import<'id>(&self, brand: &Brand<'id>) -> Self::Handles<'id> {
        // Safety: guaranteed by Mapping contract.
        unsafe { Handle::new(brand, self.mapping_id(), self.desc()) }
    }
}

/// Buffers whose mappings belong to the given Host1x channel.
///
/// # Safety
/// [`MappedTo::is_mapped_to`] may return `true` only if every mapping created
/// by [`Import::import`] is valid in `channel`.
pub unsafe trait MappedTo<Ch: Channel>: Import {
    fn is_mapped_to(&self, channel: &Ch) -> bool;
}

macro_rules! impl_import {
    ($(($B:ident, $idx:tt)),+) => {
        impl<$($B: Import),+> sealed::SealedImport for ($($B),+,) {}
        impl<$($B: Import),+> Import for ($($B),+,) {
            type Handles<'id> = ($($B::Handles<'id>),+,);

            fn import<'id>(&self, brand: &Brand<'id>) -> Self::Handles<'id> {
                ($(self.$idx.import(brand)),+,)
            }
        }
        unsafe impl<Ch: Channel, $($B: MappedTo<Ch>),+> MappedTo<Ch> for ($($B),+,) {
            fn is_mapped_to(&self, channel: &Ch) -> bool {
                true $(&& self.$idx.is_mapped_to(channel))+
            }
        }
    };
}
#[rustfmt::skip] impl_import!((B0, 0));
#[rustfmt::skip] impl_import!((B0, 0), (B1, 1));
#[rustfmt::skip] impl_import!((B0, 0), (B1, 1), (B2, 2));
#[rustfmt::skip] impl_import!((B0, 0), (B1, 1), (B2, 2), (B3, 3));
#[rustfmt::skip] impl_import!((B0, 0), (B1, 1), (B2, 2), (B3, 3), (B4, 4));
#[rustfmt::skip] impl_import!((B0, 0), (B1, 1), (B2, 2), (B3, 3), (B4, 4), (B5, 5));
#[rustfmt::skip] impl_import!((B0, 0), (B1, 1), (B2, 2), (B3, 3), (B4, 4), (B5, 5), (B6, 6));
#[rustfmt::skip] impl_import!((B0, 0), (B1, 1), (B2, 2), (B3, 3), (B4, 4), (B5, 5), (B6, 6), (B7, 7));

mod sealed {
    pub trait SealedImport {}
    pub trait Sealed<'id, C, In> {}
}

/// A description of task sequence.
///
/// A pipeline is one of:
/// - a closure taking output of the previous stage (or buffer handles if
///   it's the first stage) and returning a [`Task`],
/// - a tuple of pipelines, executed in order, each taking output of the
///   previous one.
///
/// There is no empty pipeline, so a job always runs at least one task and its
/// command sequence is never empty.
///
/// Since tuples are pipelines themselves, they can be nested to build
/// sequences longer than supported tuple size, or to group related tasks.
///
/// This trait is sealed, the closure-returning-[`Task`] form is the only way
/// to add commands to a pipeline.
pub trait Pipeline<'id, C: Client, In>: sealed::Sealed<'id, C, In> {
    /// What the last stage gives back.
    type Output;

    /// Build command sequence for the pipeline.
    fn build(self, input: In) -> (impl CommandSequence<C>, Self::Output);
}

impl<'id, C: Client, In, F: FnOnce(In) -> T, T: Task<'id, Client = C>> sealed::Sealed<'id, C, In>
    for F
{
}
impl<'id, C: Client, In, F: FnOnce(In) -> T, T: Task<'id, Client = C>> Pipeline<'id, C, In> for F {
    type Output = T::Output;

    fn build(self, input: In) -> (impl CommandSequence<C>, Self::Output) {
        self(input).split()
    }
}

// (P0, P1, .., Pn) is built as P0 followed by (P1, .., Pn). Command sequences
// are nested in pairs so only the 2-tuple CommandSequence impl is required.
macro_rules! impl_pipeline {
    ($P0:ident) => {
        impl<'id, C: Client, In, $P0: Pipeline<'id, C, In>> sealed::Sealed<'id, C, In> for ($P0,) {}
        impl<'id, C: Client, In, $P0: Pipeline<'id, C, In>> Pipeline<'id, C, In> for ($P0,) {
            type Output = $P0::Output;

            fn build(self, input: In) -> (impl CommandSequence<C>, Self::Output) {
                self.0.build(input)
            }
        }
    };
    ($P0:ident, $($P:ident),+) => {
        impl<'id, C: Client, In, $P0: Pipeline<'id, C, In>, $($P),+> sealed::Sealed<'id, C, In>
            for ($P0, $($P),+)
        where
            ($($P,)+): Pipeline<'id, C, $P0::Output>,
        {
        }
        impl<'id, C: Client, In, $P0: Pipeline<'id, C, In>, $($P),+> Pipeline<'id, C, In>
            for ($P0, $($P),+)
        where
            ($($P,)+): Pipeline<'id, C, $P0::Output>,
        {
            type Output = <($($P,)+) as Pipeline<'id, C, $P0::Output>>::Output;

            #[allow(non_snake_case)]
            fn build(self, input: In) -> (impl CommandSequence<C>, Self::Output) {
                let ($P0, $($P),+) = self;
                let (first, output) = $P0.build(input);
                let (rest, output) = ($($P,)+).build(output);
                ((first, rest), output)
            }
        }
        impl_pipeline!($($P),+);
    };
}
impl_pipeline!(
    P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P14, P15
);

/// Create a [`Job`] along with its brand.
///
/// `let job = job!(buffers, pipeline);` is equivalent to:
///
/// ```ignore
/// let guard = guard!();
/// let job = Job::new(guard, buffers, pipeline);
/// ```
///
/// The brand is created with `super let`, so it lives as long as the job does
/// when the macro is used as a `let` initializer, or until the end of the
/// statement when used as a temporary, e.g. `queue.submit(&mut fence, job!(..))`.
/// Callers don't need to enable `#![feature(super_let)]`.
///
/// # Examples
///
/// ```
/// # mod mock { include!("job/mock.rs"); } use mock::*;
/// # use host1x_core::{job, task::Task};
/// let job = job!((MockBuffer(1), MockBuffer(2)), (
///     |(a, b)| Write(a).with(b),
///     |(a, b)| Write(b).with(a),
/// ));
/// let (a, b) = job.into_buffers();
/// assert_eq!((a.0, b.0), (1, 2));
/// ```
///
/// Handles still can't be used with a different job:
///
/// ```compile_fail,E0716
/// # mod mock { include!("job/mock.rs"); } use mock::*;
/// # use host1x_core::job;
/// let _ = job!((MockBuffer(1), MockBuffer(2)), |(a, c)| {
///     let _ = job!(MockBuffer(3), |_b| Write(a));
///     Write(c)
/// });
/// ```
#[macro_export]
#[allow_internal_unstable(super_let)]
macro_rules! job {
    ($buffers:expr, $pipeline:expr $(,)?) => {{
        // `super let` is required to extend lifetime of the brand created by
        // `guard!()` past this block.
        super let guard = $crate::job::guard!();
        $crate::job::Job::new(guard, $buffers, $pipeline)
    }};
}

/// A sequence of GPU tasks being built, together with the buffers it uses.
///
/// See [module documentation](self).
///
/// # Compile-time guarantees
///
/// Examples below use following mock buffer and task:
///
/// ```
/// # use gr2d_reg::{Client as Gr2d, DSTBA};
/// # use host1x_buffer::{Buffer, Layout, LinearDesc, WritableBuffer, access::ReadWrite};
/// # use host1x_core::{cmd::Mask, cmdseq::CommandSequence, job::*, task::Task};
/// struct MockBuffer(u32);
/// impl Layout for MockBuffer {
///     fn size(&self) -> u32 {
///         0
///     }
/// }
/// impl Buffer for MockBuffer {
///     type Desc = LinearDesc;
///     fn desc(&self) -> LinearDesc {
///         LinearDesc::new(0)
///     }
/// }
/// unsafe impl Mapping for MockBuffer {
///     type Client = Gr2d;
///     type Access = ReadWrite;
///     fn mapping_id(&self) -> u32 {
///         self.0
///     }
/// }
///
/// struct Write<B>(B);
/// unsafe impl<'id, B: WritableBuffer<'id, Gr2d>> Task<'id> for Write<B> {
///     type Client = Gr2d;
///     type Output = B;
///     fn split(self) -> (impl CommandSequence<Gr2d>, B) {
///         let mut buf = self.0;
///         (Mask::<1, 1, Gr2d, _>::new((DSTBA::write(&mut buf),)), buf)
///     }
/// }
///
/// let guard = guard!();
/// let job = Job::<Gr2d, _, _>::new(
///     guard,
///     (MockBuffer(1), MockBuffer(2)),
///     (|(a, b)| Write(a).with(b), |(a, b)| Write(b).with(a)),
/// );
/// ```
///
/// Handles can't be used with a different job:
///
/// ```compile_fail,E0716
/// # use gr2d_reg::Client as Gr2d;
/// # mod mock { include!("job/mock.rs"); } use mock::*;
/// # use host1x_core::job::*;
/// let guard_a = guard!();
/// let guard_b = guard!();
/// let _ = Job::<Gr2d, _, _>::new(guard_a, (MockBuffer(1), MockBuffer(2)), |(a, c)| {
///     let _ = Job::<Gr2d, _, _>::new(guard_b, MockBuffer(3), |_b| Write(a));
///     Write(c)
/// });
/// ```
///
/// Handles can't outlive the job:
///
/// ```compile_fail,E0515
/// # use gr2d_reg::Client as Gr2d;
/// # use host1x_buffer::{LinearDesc, access::ReadWrite};
/// # mod mock { include!("job/mock.rs"); } use mock::*;
/// # use host1x_core::job::*;
/// fn escape() -> Option<Handle<'static, Gr2d, LinearDesc, ReadWrite>> {
///     let guard = guard!();
///     let mut leaked = None;
///     let _ = Job::<Gr2d, _, _>::new(guard, (MockBuffer(1), MockBuffer(2)), |(a, b)| {
///         leaked = Some(a);
///         Write(b)
///     });
///     leaked
/// }
/// ```
///
/// A job must own at least one buffer:
///
/// ```compile_fail,E0277
/// # use gr2d_reg::Client as Gr2d;
/// # use host1x_core::{cmd::SetClass, cmdseq::CommandSequence, job, task::Task};
/// struct Nop;
/// unsafe impl<'id> Task<'id> for Nop {
///     type Client = Gr2d;
///     type Output = ();
///     fn split(self) -> (impl CommandSequence<Gr2d>, ()) {
///         (SetClass::<_, _, Gr2d, _>::new(), ())
///     }
/// }
///
/// let _ = job!((), |()| Nop);
/// ```
///
/// Pipelines can't be empty:
///
/// ```compile_fail,E0277
/// # mod mock { include!("job/mock.rs"); } use mock::*;
/// # use host1x_core::job;
/// let _ = job!(MockBuffer(1), ());
/// ```
///
/// Read-only buffers can't be written to:
///
/// ```compile_fail,E0277
/// # use gr2d_reg::Client as Gr2d;
/// # use host1x_buffer::{Buffer, Layout, LinearDesc, access::ReadOnly};
/// # mod mock { include!("job/mock.rs"); } use mock::*;
/// # use host1x_core::job::*;
/// struct RoBuffer(u32);
/// impl Layout for RoBuffer {
///     fn size(&self) -> u32 {
///         0
///     }
/// }
/// impl Buffer for RoBuffer {
///     type Desc = LinearDesc;
///     fn desc(&self) -> LinearDesc {
///         LinearDesc::new(0)
///     }
/// }
/// unsafe impl Mapping for RoBuffer {
///     type Client = Gr2d;
///     type Access = ReadOnly;
///     fn mapping_id(&self) -> u32 {
///         self.0
///     }
/// }
///
/// let guard = guard!();
/// let _ = Job::<Gr2d, _, _>::new(guard, RoBuffer(1), |buf| Write(buf));
/// ```
#[must_use = "job does nothing unless submitted"]
pub struct Job<'id, C: Client, B, S> {
    _brand: Brand<'id>,
    buffers: B,
    seq: S,
    _client: PhantomData<fn() -> C>,
}
impl<'id, C: Client, B: Import> Job<'id, C, B, ()> {
    /// Take ownership of `buffers` and build the task sequence described by `pipeline`.
    ///
    /// The pipeline receives handles for the buffers. See [`Pipeline`].
    pub fn new<P: Pipeline<'id, C, B::Handles<'id>>>(
        guard: Guard<'id>,
        buffers: B,
        pipeline: P,
    ) -> Job<'id, C, B, impl CommandSequence<C>> {
        let brand = Brand(guard.into());
        let handles = buffers.import(&brand);
        // Handles left over at the end of the pipeline are useless outside of
        // the job, so drop them.
        let (seq, _) = pipeline.build(handles);
        Job {
            _brand: brand,
            buffers,
            seq,
            _client: PhantomData,
        }
    }
}
impl<'id, C: Client, B, S: CommandSequence<C>> Job<'id, C, B, S> {
    /// Discard the sequence and get the buffers back.
    pub fn into_buffers(self) -> B {
        self.buffers
    }

    /// Split into buffers and command sequence, for submission to the GPU.
    ///
    /// # Panics
    /// If any of the buffers is not mapped to `channel`.
    ///
    /// # Safety
    /// Caller must submit the command sequence only to `channel`, and must not
    /// give access to the buffers until the GPU is done executing the commands.
    pub unsafe fn into_parts<Ch: Channel<Client = C>>(self, channel: &Ch) -> (B, S)
    where
        B: MappedTo<Ch>,
    {
        assert!(
            self.buffers.is_mapped_to(channel),
            "buffers must be mapped to the channel the job is submitted to"
        );
        (self.buffers, self.seq)
    }
}
impl<C: Client, B: fmt::Debug, S> fmt::Debug for Job<'_, C, B, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("buffers", &self.buffers)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests {
    use gr2d_reg::{Client as Gr2d, DSTBA, SRCBA};
    use host1x_buffer::{ReadableBuffer, WritableBuffer};

    use crate::{cmd::Mask, cmdseq::CommandSequence, reg::reloc::Relocation, task::Task};

    use super::{
        Job, guard,
        mock::{MockBuffer, MockChannel, Write},
    };

    /// Takes source by value too, so it can be passed on to the next stage.
    struct Copy<S, D>(S, D);
    unsafe impl<'id, S: ReadableBuffer<'id, Gr2d>, D: WritableBuffer<'id, Gr2d>> Task<'id>
        for Copy<S, D>
    {
        type Client = Gr2d;
        type Output = (S, D);

        fn split(self) -> (impl CommandSequence<Gr2d>, (S, D)) {
            let Self(src, mut dst) = self;
            (
                Mask::<2, 2, Gr2d, _>::new((DSTBA::write(&mut dst), SRCBA::read(&src))),
                (src, dst),
            )
        }
    }

    fn seq_size<C: crate::cmdseq::CommandSequence<Gr2d>>(_: &C) -> (usize, usize) {
        (C::N_WORDS, C::N_RELOCS)
    }

    fn relocs(relocs: impl IntoIterator<Item = (u32, u32)>) -> Vec<Relocation> {
        relocs
            .into_iter()
            .map(|(patch_offset, mapping)| Relocation {
                patch_offset,
                mapping,
                buf_offset: 0,
                shift: 0,
            })
            .collect()
    }

    #[test]
    fn test_chain() {
        let guard = guard!();
        let job = Job::new(
            guard,
            (MockBuffer(7), MockBuffer(9)),
            (
                |(a, b)| Write(a).with(b),
                |(a, b)| Copy(a, b),
                |(a, b)| Write(b).with(a),
            ),
        );

        let ((a, b), seq) = unsafe { job.into_parts(&MockChannel) };
        assert_eq!((a.0, b.0), (7, 9));
        assert_eq!(seq_size(&seq), (7, 4));
        assert_eq!(
            seq.relocs().collect::<Vec<_>>(),
            relocs([(1, 7), (3, 9), (4, 7), (6, 9)])
        );
    }

    #[test]
    fn test_nested_pipeline() {
        let guard = guard!();
        let job = Job::new(
            guard,
            MockBuffer(3),
            (|a| Write(a), (|a| Write(a), |a| Write(a)), (|a| Write(a),)),
        );

        let (_, seq) = unsafe { job.into_parts(&MockChannel) };
        assert_eq!(seq_size(&seq), (8, 4));
        assert_eq!(
            seq.relocs().collect::<Vec<_>>(),
            relocs([(1, 3), (3, 3), (5, 3), (7, 3)])
        );
    }
}
