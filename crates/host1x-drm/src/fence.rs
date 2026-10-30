use std::{
    io,
    os::fd::AsRawFd as _,
    pin::Pin,
    task::{Context, Poll, ready},
};

use drm::control::{Device as _, syncobj};
use futures_util::FutureExt;
use getset::Getters;
use host1x_core::queue::Fence;
use pin_project::pin_project;

use crate::device::TegraDRM;

#[derive(Debug, Getters)]
pub struct Syncobj {
    #[get = "pub"]
    dev: TegraDRM,
    handle: syncobj::Handle,
}
impl Syncobj {
    #[instrument(skip(dev), fields(dev = dev.as_raw_fd()))]
    pub fn new(dev: &TegraDRM) -> io::Result<Self> {
        // Safety: only creates a new syncobj, which is then owned by self.
        let handle = unsafe { dev.drm() }.create_syncobj(false).unwrap();
        let handle_raw: u32 = handle.into();

        debug!(syncobj.handle = handle_raw, "syncobj created");
        Ok(Self {
            dev: dev.clone(),
            handle,
        })
    }

    pub(crate) fn wait(&mut self) -> SyncobjWait<'_> {
        SyncobjWait {
            state: WaitState::Initial,
            syncobj: self,
        }
    }

    /// Obtain raw DRM handle.
    ///
    /// # Safety
    /// Not strictly unsafe per-se, but the handle may be used in safe APIs (from [`drm`] crate)
    /// in a way that breaks safety guarantees of this crate. The caller must not use it to
    /// change the state of the syncobj, e.g. through [`TegraDRM::drm`].
    pub unsafe fn handle(&self) -> syncobj::Handle {
        self.handle
    }
}
impl Drop for Syncobj {
    fn drop(&mut self) {
        let handle_raw: u32 = self.handle.into();
        let span = debug_span!("drop", syncobj.handle = handle_raw);
        let _guard = span.enter();
        debug!("destroying syncpt");

        // Safety: destroys only the syncobj owned by self.
        let r = unsafe { self.dev.drm() }.destroy_syncobj(self.handle);
        #[cfg(debug_assertions)]
        r.unwrap()
    }
}

impl Fence for Syncobj {}

#[derive(Debug)]
#[pin_project]
pub struct SyncobjWait<'a> {
    state: WaitState,
    pub(crate) syncobj: &'a mut Syncobj,
}
impl Future for SyncobjWait<'_> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let proj = self.project();

        loop {
            match *proj.state {
                // Unfortunately eventfd with DRM sync object requires driver support,
                // and seems to be gated on syncobj timeline support, supported by hardware,
                // but not by the driver.
                WaitState::Initial => {
                    let dev = proj.syncobj.dev.clone();
                    let handle = proj.syncobj.handle;
                    // FIXME: not scoped, could possibly execute after handle has been destroyed
                    *proj.state = WaitState::JobSpawned(tokio::task::spawn_blocking(move || {
                        // Safety: waiting doesn't change the state of the syncobj.
                        unsafe { dev.drm() }.syncobj_wait(&[handle], -1, true, false)
                    }));
                }
                WaitState::JobSpawned(ref mut handle) => {
                    return Poll::Ready(ready!(handle.poll_unpin(cx)).unwrap().map(|_| ()));
                }
            }
        }
    }
}

#[derive(Debug)]
enum WaitState {
    Initial,
    JobSpawned(tokio::task::JoinHandle<io::Result<u32>>),
}
