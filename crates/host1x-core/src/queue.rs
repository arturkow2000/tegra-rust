//! Interfaces for queuing commands to the GPU.

use host1x_client::Client;

use crate::task::Task;

/// Interface for transmitting commands to the GPU.
pub trait Queue<C: Client> {
    /// A future that queues tasks to the GPU, but does not wait for their completion.
    ///
    /// See [`Queue::submit`] for details.
    type SubmitFuture<'p, T: Task<Client = C>>: Future<
        Output = Result<Self::PendingTask<'p, T>, Self::SubmitError>,
    >
    where
        Self: 'p;
    /// Task submission error.
    type SubmitError;

    /// GPU-side error while executing task.
    type TaskError;

    /// Fence for CPU-GPU synchronization.
    type Fence: Fence;

    /// A task that is currently running on the GPU.
    type PendingTask<'p, T: Task<Client = C>>: PendingTask<T, Self::Fence, Self::TaskError>
    where
        Self: 'p;

    /// Submit a task to the GPU.
    fn submit<'r, T: Task<Client = C>>(
        &'r self,
        fence: &'r mut Self::Fence,
        task: T,
    ) -> Self::SubmitFuture<'r, T>;
}

/// A fence for synchronization between GPU and CPU.
pub trait Fence {}

/// A task that is currently running on the GPU.
pub trait PendingTask<T: Task, F: Fence, Error>:
    Future<Output = Result<T::Buffers, Error>>
{
}
