//! GPU tasks.

use host1x_client::Client;

use crate::cmdseq::CommandSequence;

#[cfg(doc)]
use crate::job::Pipeline;

/// A single unit of GPU work.
///
/// Internally this consists of sequence of one or more Host1x commands. Tasks
/// operate on buffer handles branded with `'id`, and are chained with a
/// [`Pipeline`]: the [`Task::Output`] of one task (typically the handles it
/// was given) becomes input of the next one.
///
/// # Safety
/// Tasks must not rely on the backend to catch bad commands. A kernel driver
/// may check the command stream, and an IOMMU may confine the GPU to the
/// buffers mapped by the process, but how thoroughly depends on the driver.
/// Other backends may give the GPU access to all physical memory (Tegra 20 has
/// no IOMMU). Even when contained, stray accesses can corrupt the process's
/// own memory.
///
/// Assuming that every handle branded with `'id` describes its buffer
/// accurately, executing the commands must not cause undefined behavior,
/// whatever tasks run before or after them. In particular:
///
/// - The GPU may access memory only through relocations, never through raw
///   addresses. Every relocation must target a buffer branded with `'id`, i.e.
///   one implementing [`host1x_buffer::MappedBuffer<'id, Self::Client>`].
/// - The GPU may read a buffer only through a relocation created from a
///   [`host1x_buffer::ReadableBuffer`], and write it only through one created
///   from a [`host1x_buffer::WritableBuffer`]. The access mode of each
///   relocation must cover what the GPU actually does with the register it is
///   written to.
/// - Every access must stay within the buffer, as described by the handle's
///   layout.
/// - The task must program every register that affects how the GPU addresses
///   memory (formats, sizes, strides, positions, ...) before starting an
///   operation, instead of relying on values left by previous tasks or jobs.
/// - The task must not increment syncpoints or use other Host1x class
///   methods. Synchronization is up to the queue, which declares every
///   increment it adds when submitting the job. An increment from a task
///   wouldn't be accounted for, and could signal a fence before the GPU is
///   done.
/// - The commands must keep the channel in the class of [`Task::Client`], so
///   that the queue's completion signal waits for that client.
///
/// These are the requirements known so far. The hardware isn't fully
/// documented or understood, so this contract may be incomplete, and it may
/// change in the future as more is learned about the hardware.
pub unsafe trait Task<'id>: Sized {
    type Client: Client;
    type Output;

    /// Split the task into its commands and the buffer handles it gives back.
    fn split(self) -> (impl CommandSequence<Self::Client>, Self::Output);

    /// Pass `extra` along with the task output to the next stage of a pipeline.
    ///
    /// Useful for keeping handles of buffers that the task doesn't use.
    fn with<X>(self, extra: X) -> With<Self, X> {
        With { task: self, extra }
    }
}

/// A task with additional data passed through. See [`Task::with`].
#[derive(Debug)]
pub struct With<T, X> {
    task: T,
    extra: X,
}
// Safety: commands come only from the inner task.
unsafe impl<'id, T: Task<'id>, X> Task<'id> for With<T, X> {
    type Client = T::Client;
    type Output = (T::Output, X);

    fn split(self) -> (impl CommandSequence<Self::Client>, Self::Output) {
        let (seq, output) = self.task.split();
        (seq, (output, self.extra))
    }
}
