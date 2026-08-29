use crate::Error;

/// Block device storage backend.
///
/// Implement this trait to connect a flash chip, SD card, or any other block
/// device. See [`RamStorage`](crate::RamStorage) for a minimal example.
pub trait Storage {
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

        /// Read `buf.len()` bytes starting at `offset` within `block`.
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error>;

    /// Write `data` starting at `offset` within `block`.
    ///
    /// The block must have been erased before writing.
    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error>;

    /// Erase `block`, resetting all bytes to the erased state (typically `0xFF`).
    fn erase(&mut self, block: u32) -> Result<(), Error>;

    /// Flush pending writes. The default implementation is a no-op.
    fn sync(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[repr(transparent)]
pub(crate) struct SS<S: Storage>(pub S);

impl<'a, S: Storage> littlefs_rust_core::Storage for SS<S> {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), littlefs_rust_core::error::Error> {
        self.0.read(block, offset, buf)
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), littlefs_rust_core::error::Error> {
        self.0.write(block, offset, data)
    }

    fn erase(&mut self, block: u32) -> Result<(), littlefs_rust_core::error::Error> {
        self.0.erase(block)
    }

    fn sync(&mut self) -> Result<(), littlefs_rust_core::error::Error> {
        self.0.sync()
    }
}