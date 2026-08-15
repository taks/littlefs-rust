//! Block device config. Per lfs.h struct lfs_config.
//! Callbacks use raw function pointers for C-compatible layout.

#![allow(non_camel_case_types)]

use core::ops::Deref;

use crate::{
    error::Error,
    types::{lfs_block_t, lfs_off_t, lfs_size_t},
};

/// Read callback: (cfg, block, off, buffer, size) -> 0 or negative error
pub type lfs_read_t<T> = fn(&LfsConfig<T>, lfs_block_t, lfs_off_t, &mut [u8]) -> Result<(), Error>;

/// Prog callback: (cfg, block, off, buffer, size) -> 0 or negative error
pub type lfs_prog_t<T> = fn(&LfsConfig<T>, lfs_block_t, lfs_off_t, &[u8]) -> Result<(), Error>;

/// Erase callback: (cfg, block) -> 0 or negative error
pub type lfs_erase_t<T> = fn(&LfsConfig<T>, lfs_block_t) -> Result<(), Error>;

/// Sync callback: (cfg) -> 0 or negative error
pub type lfs_sync_t<T> = fn(&LfsConfig<T>) -> Result<(), Error>;

/// Per lfs.h struct lfs_config.
/// Layout matches C for potential FFI. Callbacks use Option to allow null.
#[repr(C)]
pub struct LfsConfig<T> {
    pub context: *mut core::ffi::c_void,
    pub read: Option<lfs_read_t<T>>,
    pub prog: Option<lfs_prog_t<T>>,
    pub erase: Option<lfs_erase_t<T>>,
    pub sync: Option<lfs_sync_t<T>>,
    pub read_size: lfs_size_t,
    pub prog_size: lfs_size_t,
    pub block_size: lfs_size_t,
    pub block_count: lfs_size_t,
    pub block_cycles: i32,
    pub cache_size: lfs_size_t,
    pub lookahead_size: lfs_size_t,
    pub compact_thresh: lfs_size_t,
    pub read_buffer: T,
    pub prog_buffer: T,
    pub lookahead_buffer: T,
    pub name_max: lfs_size_t,
    pub file_max: lfs_size_t,
    pub attr_max: lfs_size_t,
    pub metadata_max: lfs_size_t,
    pub inline_max: lfs_size_t,
}
