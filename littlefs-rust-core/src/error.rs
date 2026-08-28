//! Error codes. Per lfs.h enum lfs_error.
//! Negative values allow positive return values (e.g. bytes read).

/// LittleFS operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// I/O error
    Io,
    /// filesystem corrupt
    Corrupt,
    /// no such file or directory
    NoEntry,
    /// file or directory already exists
    Exists,
    /// not a directory
    NotDir,
    /// is a directory
    IsDir,
    /// directory not empty
    NotEmpty,
    /// invalid parameter
    Invalid,
    /// no space left on device
    NoSpace,
    /// out of memory
    NoMemory,
    /// no such attribute
    NoAttribute,
    /// name too long
    NameTooLong,
    FileTooBig,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl core::error::Error for Error {}

/// Positive return values for commit/orphan machinery. Per lfs.h enum lfs_error.
pub const LFS_OK_RELOCATED: i32 = 1;
pub const LFS_OK_DROPPED: i32 = 2;
pub const LFS_OK_ORPHANED: i32 = 3;
