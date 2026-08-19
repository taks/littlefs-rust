//! RAM block device for unit tests. Erase = 0xff; prog = copy; read = copy.

use core::ptr::NonNull;

use crate::{LfsConfig, error::Error, lfs_config::Storage};

/// Magic string "littlefs" in superblock blocks. Per lfs.h.
pub const MAGIC: &[u8; 8] = b"littlefs";
/// Offset of magic in first commit. Layout varies (8 or 12); this is the bypass/attr path.
pub const MAGIC_OFFSET: u32 = 12;

/// RAM block device storage.
pub struct RamStorage {
    pub data: alloc::vec::Vec<u8>,
    pub block_size: u32,
    pub _block_count: u32,
}

impl RamStorage {
    pub fn new(block_size: u32, block_count: u32) -> Self {
        let size = (block_size as usize)
            .checked_mul(block_count as usize)
            .expect("overflow");
        Self {
            data: alloc::vec![0u8; size],
            block_size,
            _block_count: block_count,
        }
    }

    pub fn block_offset(&self, block: u32) -> usize {
        (block as usize)
            .checked_mul(self.block_size as usize)
            .expect("block overflow")
    }

    pub fn read(&mut self, block: u32, off: u32, buf: &mut [u8]) {
        let base = self.block_offset(block);
        let start = base + off as usize;
        let end = start + buf.len();
        buf.copy_from_slice(&self.data[start..end]);
    }

    pub fn prog(&mut self, block: u32, off: u32, buf: &[u8]) {
        let base = self.block_offset(block);
        let start = base + off as usize;
        let end = start + buf.len();
        self.data[start..end].copy_from_slice(buf);
    }

    pub fn erase(&mut self, block: u32) {
        let base = self.block_offset(block);
        let end = base + self.block_size as usize;
        self.data[base..end].fill(0xff);
    }
}

pub const BLOCK_SIZE: u32 = 512;

impl Storage for RamStorage {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        assert!(
            !self.data.is_empty(),
            "ram_read: RamStorage.data is empty; config.context may be invalid"
        );
        self.read(block, offset, buf);
        Ok(())
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error> {
        self.prog(block, offset, data);
        Ok(())
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        self.erase(block);
        Ok(())
    }
}

/// Builds LfsConfig for the given RAM storage. Caller must set context after moving.
pub fn make_config(block_count: u32, ram: &mut RamStorage) -> LfsConfig {
    let block_size = BLOCK_SIZE;
    LfsConfig {
        context: Some(NonNull::from_mut(ram)),
        read_size: 16,
        prog_size: 16,
        block_size,
        block_count,
        block_cycles: -1,
        cache_size: block_size,
        lookahead_size: block_size,
        compact_thresh: u32::MAX,
        read_buffer: core::ptr::null_mut(),
        prog_buffer: core::ptr::null_mut(),
        lookahead_buffer: core::ptr::null_mut(),
        name_max: 255,
        file_max: 2_147_483_647,
        attr_max: 1022,
        metadata_max: 0,
        inline_max: 0,
    }
}
