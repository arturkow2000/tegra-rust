use std::{
    fmt::Debug,
    os::fd::{AsFd, AsRawFd, BorrowedFd},
    pin::Pin,
    task::{Context, Poll, ready},
};

use drm::control::{self, Device as _};
use futures_util::Stream;
use pin_project::pin_project;
use tokio::io::{self, unix::AsyncFd};

enum DrmEventStreamState {
    Poll,
    Events(control::Events),
}

#[pin_project]
pub struct DrmEventStream<T: AsFd + AsRawFd> {
    fd: AsyncFd<T>,
    state: DrmEventStreamState,
}
impl<T: AsFd + AsRawFd> DrmEventStream<T> {
    pub fn new(fd: AsyncFd<T>) -> Self {
        Self {
            fd,
            state: DrmEventStreamState::Poll,
        }
    }
}
impl<T: AsFd + AsRawFd + Debug> Debug for DrmEventStream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DrmEventStream")
            .field("fd", &self.fd)
            .finish()
    }
}
impl<T: AsFd + AsRawFd> Stream for DrmEventStream<T> {
    type Item = io::Result<drm::control::Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        struct DrmFromExternalFd<T: AsFd>(T);
        impl<T: AsFd> AsFd for DrmFromExternalFd<T> {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.as_fd()
            }
        }
        impl<T: AsFd> drm::Device for DrmFromExternalFd<T> {}
        impl<T: AsFd> control::Device for DrmFromExternalFd<T> {}

        loop {
            match self.state {
                DrmEventStreamState::Poll => {
                    let drm = DrmFromExternalFd(self.fd.as_fd());
                    let mut guard = ready!(self.fd.poll_read_ready(cx))?;
                    let events = match guard.try_io(|_| drm.receive_events()) {
                        Ok(Ok(events)) => events,
                        Ok(Err(error)) => return Poll::Ready(Some(Err(error))),
                        Err(_would_block) => continue,
                    };
                    self.state = DrmEventStreamState::Events(events);
                }
                DrmEventStreamState::Events(ref mut events) => match events.next() {
                    Some(event) => return Poll::Ready(Some(Ok(event))),
                    None => self.state = DrmEventStreamState::Poll,
                },
            }
        }
    }
}
