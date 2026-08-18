//! Block device config. Per lfs.h struct lfs_config.
//! Callbacks use raw function pointers for C-compatible layout.

#![allow(non_camel_case_types)]

use crate::{
    error::Error,
    types::{lfs_block_t, lfs_off_t, lfs_size_t},
};

/// Read callback: (cfg, block, off, buffer, size) -> 0 or negative error
pub type lfs_read_t = fn(&LfsConfig, lfs_block_t, lfs_off_t, &mut [u8]) -> Result<(), Error>;

/// Prog callback: (cfg, block, off, buffer, size) -> 0 or negative error
pub type lfs_prog_t = fn(&LfsConfig, lfs_block_t, lfs_off_t, &[u8]) -> Result<(), Error>;

/// Erase callback: (cfg, block) -> 0 or negative error
pub type lfs_erase_t = fn(&LfsConfig, lfs_block_t) -> Result<(), Error>;

/// Sync callback: (cfg) -> 0 or negative error
pub type lfs_sync_t = fn(&LfsConfig) -> Result<(), Error>;

pub trait Storage {
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

/// Per lfs.h struct lfs_config.
/// Layout matches C for potential FFI. Callbacks use Option to allow null.
#[repr(C)]
pub struct LfsConfig {
    pub context: *mut dyn Storage,
    pub read_size: lfs_size_t,
    pub prog_size: lfs_size_t,
    pub block_size: lfs_size_t,
    pub block_count: lfs_size_t,
    pub block_cycles: i32,
    pub cache_size: lfs_size_t,
    pub lookahead_size: lfs_size_t,
    pub compact_thresh: lfs_size_t,
    pub read_buffer: *mut core::ffi::c_void,
    pub prog_buffer: *mut core::ffi::c_void,
    pub lookahead_buffer: *mut core::ffi::c_void,
    pub name_max: lfs_size_t,
    pub file_max: lfs_size_t,
    pub attr_max: lfs_size_t,
    pub metadata_max: lfs_size_t,
    pub inline_max: lfs_size_t,
}
