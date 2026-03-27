use std::{
    fmt::Debug,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
};

use drm::node::constants::DRM_MAJOR;
use rustix::fs::{FileType, fstat, major};
use snafu::{ResultExt as _, ensure};

use super::error::{IOSnafu, NotDrmDevSnafu, Result};

#[derive(Debug)]
pub struct AnyDevice {
    handle: OwnedFd,
}

impl AnyDevice {
    pub fn try_from_fd(handle: OwnedFd) -> Result<Self> {
        let st = fstat(handle.as_fd()).map_err(Into::into).context(IOSnafu)?;
        let typ = FileType::from_raw_mode(st.st_mode);
        ensure!(
            typ == FileType::CharacterDevice && major(st.st_rdev) == DRM_MAJOR,
            NotDrmDevSnafu
        );
        Ok(Self { handle })
    }
}

impl AsFd for AnyDevice {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.handle.as_fd()
    }
}

impl drm::Device for AnyDevice {}
