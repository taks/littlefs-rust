//! Lookahead buffer. Per lfs.h struct lfs_lookahead.

use core::ptr::NonNull;

use crate::types::lfs_block_t;

/// Per lfs.h struct lfs_lookahead
#[repr(C)]
pub struct LfsLookahead {
    pub start: lfs_block_t,
    pub size: lfs_block_t,
    pub next: lfs_block_t,
    pub ckpoint: lfs_block_t,
    pub buffer: NonNull<[u8]>,
}

impl Default for LfsLookahead {
    fn default() -> Self {
        Self {
            start: Default::default(),
            size: Default::default(),
            next: Default::default(),
            ckpoint: Default::default(),
            buffer: NonNull::from_ref(&[]),
        }
    }
}
