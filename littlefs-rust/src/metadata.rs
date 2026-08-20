use alloc::string::String;
use bitflags::bitflags;

/// Type of a filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Dir,
}

/// Metadata for a file or directory, as returned by [`Filesystem::stat`](crate::Filesystem::stat).
#[derive(Debug, Clone)]
pub struct Metadata {
    pub file_type: FileType,
    pub size: u32,
    pub name: String,
}

/// A single entry from a directory listing.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub file_type: FileType,
    pub size: u32,
}

/// Position for [`File::seek`](crate::File::seek).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// Start of file.
    Start(u32),
    /// Current position (offset can be negative).
    Current(i32),
    /// End of file (offset can be negative).
    End(i32),
}
