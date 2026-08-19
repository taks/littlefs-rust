use littlefs_rust_core::error::Error;

use crate::RamStorage;

/// Block device storage backend.
///
/// Implement this trait to connect a flash chip, SD card, or any other block
/// device. See [`RamStorage`](crate::RamStorage) for a minimal example.
pub trait StorageWithConfig: littlefs_rust_core::Storage {
    /// Minimum size of block read in bytes. Not in superblock
    const READ_SIZE: usize;

    /// Minimum size of block write in bytes. Not in superblock
    const WRITE_SIZE: usize;

    /// Size of an erasable block in bytes, as unsigned typenum.
    /// Must be a multiple of both `READ_SIZE` and `WRITE_SIZE`.
    const BLOCK_SIZE: usize;

    /// Number of erasable blocks.
    /// Hence storage capacity is `BLOCK_COUNT * BLOCK_SIZE`
    const BLOCK_COUNT: usize;

    /// Suggested values are 100-1000, higher is more performant but
    /// less wear-leveled.  Default of -1 disables wear-leveling.
    /// Value zero is invalid, must be positive or -1.
    const BLOCK_CYCLES: isize = -1;
}
