use alloc::vec;
use alloc::vec::Vec;
use littlefs_rust_core::error::Error;

use crate::storage::Storage;

/// In-memory block device for testing and examples.
///
/// Simulates flash: erased blocks are `0xFF`, writes overwrite bytes, and
/// erase resets a block to `0xFF`. Use with [`Config`](crate::Config) and
/// [`Filesystem`](crate::Filesystem).
pub struct RamStorage<const BLOCK_SIZE: u32, const BLOCK_COUNT: u32> {
    data: Vec<u8>,
}

impl<const BLOCK_SIZE: u32, const BLOCK_COUNT: u32> RamStorage<BLOCK_SIZE, BLOCK_COUNT> {
    #[allow(clippy::new_without_default)]
    /// Create a new RAM-backed storage with the given block geometry.
    pub fn new() -> Self {
        let size = (BLOCK_SIZE as usize)
            .checked_mul(BLOCK_COUNT as usize)
            .expect("block_size * block_count overflow");
        Self {
            data: vec![0xFFu8; size],
        }
    }

    /// Return the raw storage bytes (for inspection or persistence).
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn block_size(&self) -> u32 {
        BLOCK_SIZE
    }

    pub fn block_count(&self) -> u32 {
        BLOCK_COUNT
    }

    fn offset(&self, block: u32, off: u32) -> usize {
        (block as usize) * (BLOCK_SIZE as usize) + (off as usize)
    }
}

impl<const BLOCK_SIZE: u32, const BLOCK_COUNT: u32> Storage
    for RamStorage<BLOCK_SIZE, BLOCK_COUNT>
{
    const READ_SIZE: usize = 1;
    const WRITE_SIZE: usize = 1;
    const BLOCK_SIZE: usize = BLOCK_SIZE as usize;
    const BLOCK_COUNT: usize = BLOCK_COUNT as usize;

    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        let start = self.offset(block, offset);
        let end = start + buf.len();
        if end > self.data.len() {
            return Err(Error::Io);
        }
        buf.copy_from_slice(&self.data[start..end]);
        Ok(())
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error> {
        let start = self.offset(block, offset);
        let end = start + data.len();
        if end > self.data.len() {
            return Err(Error::Io);
        }
        self.data[start..end].copy_from_slice(data);
        Ok(())
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        let start = self.offset(block, 0);
        let end = start + BLOCK_SIZE as usize;
        if end > self.data.len() {
            return Err(Error::Io);
        }
        self.data[start..end].fill(0xFF);
        Ok(())
    }
}
