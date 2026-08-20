/// Type of a filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Dir,
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
