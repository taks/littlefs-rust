//! Error codes. Per lfs.h enum lfs_error.
//! Negative values allow positive return values (e.g. bytes read).
//!

/// LittleFS operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// I/O error
    Io = -5,
    /// filesystem corrupt
    Corrupt = -84,
    /// no such file or directory
    NoEntry = -2,
    /// file or directory already exists
    Exists = -17,
    /// not a directory
    NotDir = -20,
    /// is a directory
    IsDir = -21,
    /// directory not empty
    NotEmpty = -39,
    /// invalid parameter
    Invalid = -22,
    /// no space left on device
    NoSpace = -28,
    /// out of memory
    NoMemory = -12,
    /// no such attribute
    NoAttribute = -61,
    /// name too long
    NameTooLong = -36,
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
