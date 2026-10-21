use std::{
    io,
    pin::Pin,
    task::{Context, Poll, ready},
};

use futures_util::FutureExt;
use host1x_client::Client;
use host1x_core::{
    cmd::WriteNoIncr,
    cmdseq::CommandSequence,
    job::{Job, MappedTo},
    reg::class::INCR_SYNCPT,
};
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
    type Channel = Channel<C>;
    type SubmitFuture<'p, B: MappedTo<Channel<C>>, S: CommandSequence<C>> = Submit<'p, C, B, S>;
    type SubmitError = io::Error;
    type TaskError = io::Error;
    type Fence = Syncobj;
    type PendingTask<'p, B> = Pending<'p, B>;

    #[inline(always)]
    fn submit<'r, B: MappedTo<Channel<C>>, S: CommandSequence<C>>(
        &'r self,
        fence: &'r mut Self::Fence,
        job: Job<'_, C, B, S>,
    ) -> Self::SubmitFuture<'r, B, S> {
        // Safety: commands are submitted only to self.channel, buffers are kept
        // by Submit and then Pending, until the fence signals completion.
        let (buffers, seq) = unsafe { job.into_parts(self.channel()) };
        Submit {
            state: Some((buffers, seq, fence.wait())),
            channel: self.channel(),
            syncpoint: &self.syncpoint,
        }
    }
}

#[derive(Debug)]
#[pin_project]
pub struct Submit<'a, C: Client, B, S> {
    channel: &'a Channel<C>,
    syncpoint: &'a SyncPoint,
    state: Option<(B, S, SyncobjWait<'a>)>,
}
impl<'a, C: Client, B, S: CommandSequence<C>> Future for Submit<'a, C, B, S> {
    type Output = io::Result<Pending<'a, B>>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let proj = self.project();

        // FIXME: submit is blocking, we should do this from a blocking thread pool,
        // but currently CommandSequence (and many other types) are !Send.
        let (buffers, seq, mut wait) = proj.state.take().unwrap();
        proj.channel.submit(
            (
                seq,
                WriteNoIncr::new([
                    INCR_SYNCPT::COND::OP_DONE + INCR_SYNCPT::INDX.val(proj.syncpoint.id())
                ]),
            ),
            proj.syncpoint,
            None,
            Some(&mut wait.syncobj),
        )?;

        Poll::Ready(Ok(Pending {
            buffers: Some(buffers),
            wait,
        }))
    }
}

#[derive(Debug)]
#[pin_project]
pub struct Pending<'a, B> {
    buffers: Option<B>,
    wait: SyncobjWait<'a>,
}
impl<B> host1x_core::queue::PendingTask<B, Syncobj, io::Error> for Pending<'_, B> {}
impl<B> Future for Pending<'_, B> {
    type Output = io::Result<B>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let proj = self.project();
        ready!(proj.wait.poll_unpin(cx))?;
        Poll::Ready(Ok(proj.buffers.take().unwrap()))
    }
}
