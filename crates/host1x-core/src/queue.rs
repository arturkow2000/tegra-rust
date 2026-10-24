//! Interfaces for queuing commands to the GPU.

use host1x_client::Client;

use crate::{
    cmdseq::CommandSequence,
    job::{Job, MappedTo},
};

/// A Host1x channel.
///
/// Channels are the only thing buffers can be mapped to, so this is what
/// [`MappedTo`] checks buffers against. Implemented by backends for their
/// channel types.
pub trait Channel {
    /// Host1x client programmed through the channel.
    type Client: Client;
}

/// Interface for transmitting commands to the GPU.
pub trait Queue<C: Client> {
    /// Channel the queue submits commands to. Buffers must be mapped to it.
    type Channel: Channel<Client = C>;

    /// A future that queues commands to the GPU, but does not wait for their completion.
    ///
    /// See [`Queue::submit`] for details.
    type SubmitFuture<'p, B: MappedTo<Self::Channel>, S: CommandSequence<C>>: Future<
        Output = Result<Self::PendingTask<'p, B>, Self::SubmitError>,
    >
    where
        Self: 'p;
    /// Task submission error.
    type SubmitError;

    /// GPU-side error while executing task.
    type TaskError;

    /// Fence for CPU-GPU synchronization.
    type Fence: Fence;

    /// Commands that are currently running on the GPU.
    type PendingTask<'p, B>: PendingTask<B, Self::Fence, Self::TaskError>
    where
        Self: 'p;

    /// Submit all tasks from the job to the GPU.
    ///
    /// Buffers are given back once the GPU is done.
    ///
    /// # Panics
    /// If the buffers owned by the job are not mapped to the queue's channel.
    fn submit<'r, B: MappedTo<Self::Channel>, S: CommandSequence<C>>(
        &'r self,
        fence: &'r mut Self::Fence,
        job: Job<'_, C, B, S>,
    ) -> Self::SubmitFuture<'r, B, S>;
}

/// A fence for synchronization between GPU and CPU.
pub trait Fence {}

/// Commands that are currently running on the GPU. Resolves to the buffers they used.
pub trait PendingTask<B, F: Fence, Error>: Future<Output = Result<B, Error>> {}
