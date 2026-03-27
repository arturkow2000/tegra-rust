use std::{
    fmt::Debug,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
    path::Path,
    sync::Arc,
};

use drm::Device as _;
use rustix::fs::{Mode, OFlags};
use snafu::{ResultExt as _, ensure};
use tracing::{Span, field};

use self::error::{IOSnafu, NotTegraDevSnafu, Result};

mod any;
mod error;

#[derive(Debug)]
struct TegraDRMImpl {
    dev: any::AnyDevice,
}

#[derive(Debug, Clone)]
pub struct TegraDRM {
    imp: Arc<TegraDRMImpl>,
}
impl TegraDRM {
    /// Obtain [`TegraDRM`] object from raw file descriptors. Validates that the file
    /// descriptor refers to supported Tegra GPU device, which currently is
    /// Tegra DRM device when managed by Linux's `tegra` driver.
    #[instrument(skip(fd), fields(
        fd = field::display(fd.as_raw_fd())
    ))]
    pub fn try_from_fd(fd: OwnedFd) -> Result<Self> {
        let dev = any::AnyDevice::try_from_fd(fd)?;
        let driver = dev.get_driver().context(IOSnafu)?;
        // Ensure the device is driven by Linux `tegra` driver.
        // This check is critical and large portion of this crate depends on this to
        // provide safe APIs.
        //
        // Without this check we may not implement Device trait.
        ensure!(driver.name() == "tegra", NotTegraDevSnafu);
        Ok(Self {
            imp: Arc::new(TegraDRMImpl { dev }),
        })
    }

    /// Open Tegra device by path.
    #[instrument(skip(path), fields(
        path = field::Empty
    ))]
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let span = Span::current();
        span.record("path", field::display(path.display()));

        let fd = rustix::fs::open(path, OFlags::RDWR, Mode::empty())
            .map_err(Into::into)
            .context(IOSnafu)?;
        Self::try_from_fd(fd)
    }
}

impl AsFd for TegraDRM {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.imp.dev.as_fd()
    }
}

impl drm::Device for TegraDRM {}
