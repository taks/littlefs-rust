//! Block device config. Per lfs.h struct lfs_config.
//! Callbacks use raw function pointers for C-compatible layout.

#![allow(non_camel_case_types)]

use core::ptr::NonNull;

use crate::{error::Error, types::lfs_size_t};

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
    pub context: Option<NonNull<dyn Storage>>,
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
