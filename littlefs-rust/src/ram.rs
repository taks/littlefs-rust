use littlefs_rust_core::error::Error;

use crate::storage::Storage;

/// In-memory block device for testing and examples.
///
/// Simulates flash: erased blocks are `0xFF`, writes overwrite bytes, and
/// erase resets a block to `0xFF`. Use with [`Config`](crate::Config) and
/// [`Filesystem`](crate::Filesystem).
pub struct RamStorage<const BLOCK_SIZE: usize, const BLOCK_COUNT: usize> {
    data: [[u8; BLOCK_SIZE]; BLOCK_COUNT],
}

impl<const BLOCK_SIZE: usize, const BLOCK_COUNT: usize> RamStorage<BLOCK_SIZE, BLOCK_COUNT> {
    #[allow(clippy::new_without_default)]
    /// Create a new RAM-backed storage with the given block geometry.
    pub fn new() -> Self {
        Self {
            data: [[0u8; BLOCK_SIZE]; BLOCK_COUNT],
        }
    }
}

impl<const BLOCK_SIZE: usize, const BLOCK_COUNT: usize> Storage
    for RamStorage<BLOCK_SIZE, BLOCK_COUNT>
{
    const READ_SIZE: usize = 32;
    const WRITE_SIZE: usize = 32;
    const BLOCK_SIZE: usize = BLOCK_SIZE;
    const BLOCK_COUNT: usize = BLOCK_COUNT;
    type CACHE_SIZE = hybrid_array::sizes::U32;
    type LOOKAHEAD_SIZE = hybrid_array::sizes::U1;

    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        buf.copy_from_slice(&self.data[block as usize][(offset as usize)..(offset as usize + buf.len())]);
        Ok(())
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error> {
        self.data[block as usize][(offset as usize)..(offset as usize + data.len())].copy_from_slice(data);
        Ok(())
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        self.data[block as usize].fill(0xFF);
        Ok(())
    }
}
