use std::{
    io,
    pin::Pin,
    task::{Context, Poll, ready},
};

use futures_util::FutureExt;
use host1x_client::Client;
use host1x_core::{cmd::WriteNoIncr, reg::class::INCR_SYNCPT, task::Task};
use pin_project::pin_project;

use crate::fence::{Syncobj, SyncobjWait};

use super::{channel::Channel, syncpt::SyncPoint};

#[derive(Debug)]
pub struct Queue<C: Client> {
    channel: Channel<C>,
    syncpoint: SyncPoint,
}
impl<C: Client> Queue<C> {
    #[inline(always)]
    pub fn channel(&self) -> &Channel<C> {
        &self.channel
    }

    #[inline(always)]
    pub fn new(channel: Channel<C>, syncpoint: SyncPoint) -> Self {
        Self { channel, syncpoint }
    }
}
impl<C: Client> host1x_core::queue::Queue<C> for Queue<C> {
    type SubmitFuture<'p, T: Task<Client = C>> = Submit<'p, T>;
    type SubmitError = io::Error;
    type TaskError = io::Error;
    type Fence = Syncobj;
    type PendingTask<'p, T: Task<Client = C>> = Pending<'p, T>;

    #[inline(always)]
    fn submit<'r, T: Task<Client = C>>(
        &'r self,
        fence: &'r mut Self::Fence,
        task: T,
    ) -> Self::SubmitFuture<'r, T> {
        Submit {
            task_wait: Some((task, fence.wait())),
            channel: self.channel(),
            syncpoint: &self.syncpoint,
        }
    }
}

#[derive(Debug)]
#[pin_project]
pub struct Submit<'a, T: Task> {
    channel: &'a Channel<T::Client>,
    syncpoint: &'a SyncPoint,
    task_wait: Option<(T, SyncobjWait<'a>)>,
}
impl<'a, T: Task> Future for Submit<'a, T> {
    type Output = io::Result<Pending<'a, T>>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let proj = self.project();

        // FIXME: submit is blocking, we should do this from a blocking thread pool,
        // but currently CommandSequence (and many other types) are !Send.
        let (mut task, mut wait) = proj.task_wait.take().unwrap();
        proj.channel.submit(
            (
                task.cmdseq(),
                WriteNoIncr::new([
                    INCR_SYNCPT::COND::OP_DONE + INCR_SYNCPT::INDX.val(proj.syncpoint.id())
                ]),
            ),
            proj.syncpoint,
            None,
            Some(&mut wait.syncobj),
        )?;

        Poll::Ready(Ok(Pending {
            task: Some(task),
            wait,
        }))
    }
}

#[derive(Debug)]
#[pin_project]
pub struct Pending<'a, T: Task> {
    task: Option<T>,
    wait: SyncobjWait<'a>,
}
impl<T: Task> host1x_core::queue::PendingTask<T, Syncobj, io::Error> for Pending<'_, T> {}
impl<T: Task> Future for Pending<'_, T> {
    type Output = io::Result<T::Buffers>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        ready!(self.wait.poll_unpin(cx))?;
        Poll::Ready(Ok(self.task.take().unwrap().buffers()))
    }
}
