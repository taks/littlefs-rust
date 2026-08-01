//! Error codes. Per lfs.h enum lfs_error.
//! Negative values allow positive return values (e.g. bytes read).

/// LittleFS operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Io,
    Corrupt,
    NoEntry,
    Exists,
    NotDir,
    IsDir,
    NotEmpty,
    Invalid,
    NoSpace,
    NoMemory,
    NoAttribute,
    NameTooLong,
    FileTooBig,
}

/// Positive return values for commit/orphan machinery. Per lfs.h enum lfs_error.
pub const LFS_OK_RELOCATED: i32 = 1;
pub const LFS_OK_DROPPED: i32 = 2;
pub const LFS_OK_ORPHANED: i32 = 3;
