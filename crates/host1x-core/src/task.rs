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
/// Every relocation in the command sequence returned by [`Task::split`] must
/// target a buffer branded with `'id`, i.e. one implementing
/// [`host1x_buffer::MappedBuffer<'id, Self::Client>`].
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
