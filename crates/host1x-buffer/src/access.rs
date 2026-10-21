//! Markers describing how a Host1x client may access a buffer.

mod sealed {
    pub trait Sealed {}
}

/// Read-only access.
#[derive(Debug, Clone, Copy)]
pub struct ReadOnly;

/// Write-only access.
#[derive(Debug, Clone, Copy)]
pub struct WriteOnly;

/// Read-write access.
#[derive(Debug, Clone, Copy)]
pub struct ReadWrite;

/// Any access mode.
pub trait Access: sealed::Sealed {}

/// Access mode that allows reading.
pub trait Readable: Access {}

/// Access mode that allows writing.
pub trait Writable: Access {}

impl sealed::Sealed for ReadOnly {}
impl Access for ReadOnly {}
impl Readable for ReadOnly {}

impl sealed::Sealed for WriteOnly {}
impl Access for WriteOnly {}
impl Writable for WriteOnly {}

impl sealed::Sealed for ReadWrite {}
impl Access for ReadWrite {}
impl Readable for ReadWrite {}
impl Writable for ReadWrite {}
