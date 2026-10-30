use std::{
    fmt::Debug,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    path::Path,
    sync::Arc,
};

use drm::Device as _;
use rustix::fs::{Mode, OFlags};
use snafu::{ResultExt as _, ensure};
use tracing::{Span, field};

use self::error::{IOSnafu, NotTegraDevSnafu, Result, SyncobjNotSupportedSnafu};

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
    ///
    /// Use [`TegraDRM::open`] to avoid the safety requirements below.
    ///
    /// # Safety
    /// GEM buffers, syncobjs and other objects created on the device are identified by
    /// handles local to the open file description of `fd`. Objects of this crate rely on
    /// nobody else closing, signalling or exporting the handles they own. The caller must
    /// ensure that no other file descriptor referring to the same open file description
    /// (e.g. created with `dup()` or passed to another process) is used to manipulate DRM
    /// objects while the device or any object created on it is alive.
    #[instrument(skip(fd), fields(
        fd = field::display(fd.as_raw_fd())
    ))]
    pub unsafe fn try_from_fd(fd: OwnedFd) -> Result<Self> {
        let dev = any::AnyDevice::try_from_fd(fd)?;
        let driver = dev.get_driver().context(IOSnafu)?;
        // Ensure the device is driven by Linux `tegra` driver.
        // This check is critical and large portion of this crate depends on this to
        // provide safe APIs.
        ensure!(driver.name() == "tegra", NotTegraDevSnafu);
        ensure!(
            dev.get_driver_capability(drm::DriverCapability::SyncObj)
                .unwrap_or(0)
                != 0,
            SyncobjNotSupportedSnafu
        );
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

        let fd = rustix::fs::open(path, OFlags::RDWR | OFlags::CLOEXEC, Mode::empty())
            .map_err(Into::into)
            .context(IOSnafu)?;
        // Safety: the file has just been opened, the only file descriptor referring to it is `fd`.
        unsafe { Self::try_from_fd(fd) }
    }

    /// File descriptor of the device.
    ///
    /// Deliberately not public, and neither are [`AsFd`] nor [`drm::Device`] implemented:
    /// safe APIs of [`drm`] and [`drm_ffi`] would allow to close, signal or export handles owned
    /// by objects of this crate. See [`TegraDRM::try_from_fd`].
    pub(crate) fn as_fd(&self) -> BorrowedFd<'_> {
        self.imp.dev.as_fd()
    }

    /// DRM API of the device.
    ///
    /// # Safety
    /// [`drm`] and [`drm_ffi`] crates allow direct manipulation of many DRM objects
    /// (e.g. signaling of syncobjs), thus it's possible to break all safety guarantees of
    /// this crate.
    ///
    /// The caller must not use the returned device, or anything obtained from it (e.g. a
    /// duplicated file descriptor), to manipulate DRM objects owned by objects of this crate
    /// (GEM buffers, syncobjs, channels, ...).
    pub unsafe fn drm(&self) -> &impl drm::control::Device {
        &self.imp.dev
    }
}
// Do NOT implement AsFd.
//
// drm and drm_ffi crates provide safe APIs which allow direct manipulation of
// DRM-local (bound to DRM fd) handles (e.g. syncobj's, GEM handles, etc),
// breaking all safety guarantees of this crate.
impl AsRawFd for TegraDRM {
    fn as_raw_fd(&self) -> RawFd {
        self.imp.dev.as_fd().as_raw_fd()
    }
}
