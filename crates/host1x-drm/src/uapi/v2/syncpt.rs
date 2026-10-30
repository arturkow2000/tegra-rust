use std::{io, os::fd::AsRawFd, time::Duration};

use getset::Getters;
use rustix::time::{ClockId, clock_gettime};
use tracing::field;

use crate::device::TegraDRM;

use super::ioctl;

#[derive(Debug, Getters)]
pub struct SyncPoint {
    #[get = "pub"]
    dev: TegraDRM,
    handle: u32,
}
impl SyncPoint {
    /// Exclusively allocate hardware sync point.
    ///
    /// Note that hardware has limited number of sync points and this may fail
    /// if syncpoints are exhausted. Due to how DRM driver works it is possible
    /// to permanently starve all other DRM clients by allocating all syncpoints
    /// for ourselves.
    #[instrument(skip(dev), fields(
        dev = dev.as_raw_fd()
    ))]
    pub fn alloc(dev: &TegraDRM) -> io::Result<Self> {
        let handle = ioctl::syncpt_alloc(dev)
            .inspect_err(|error| error!(error = field::display(error), "syncpt alloc fail"))?;
        debug!(syncpt.id = handle, "allocated syncpt");
        Ok(Self {
            dev: dev.clone(),
            handle,
        })
    }
}
impl SyncPoint {
    /// Obtain raw ID of hardware sync point.
    pub fn id(&self) -> u32 {
        self.handle
    }

    pub fn wait(&self, threshold: u32, timeout: Duration) -> io::Result<u32> {
        let now = clock_gettime(ClockId::Monotonic);
        let now = now.tv_sec * 1000000000 + now.tv_nsec;
        let timeout: i64 = timeout.as_nanos().try_into().expect("timeout overflow");

        ioctl::syncpt_wait(&self.dev, self.handle, threshold, now + timeout)
    }
}
impl Drop for SyncPoint {
    fn drop(&mut self) {
        let span = debug_span!("drop", syncpt.id = self.handle);
        let _guard = span.enter();
        debug!("releasing syncpt");

        let r = ioctl::syncpt_free(&self.dev, self.handle);
        #[cfg(debug_assertions)]
        r.unwrap();
    }
}
