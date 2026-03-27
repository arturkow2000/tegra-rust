use std::io;

use snafu::Snafu;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("not DRM device"))]
    NotDrmDev,

    #[snafu(display("not Tegra DRM device"))]
    NotTegraDev,

    #[snafu(display("unknown I/O error"))]
    IOError { source: io::Error },
}
