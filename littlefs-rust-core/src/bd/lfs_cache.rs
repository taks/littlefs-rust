//! Block cache. Per lfs.h lfs_cache_t.

use core::ptr::NonNull;

use crate::types::lfs_block_t;

/// Per lfs.h typedef struct lfs_cache
pub struct LfsCache {
    pub block: lfs_block_t,
    pub off: u32,
    pub size: u32,
    pub buffer: NonNull<[u8]>,
}

impl Default for LfsCache {
    fn default() -> Self {
        Self {
            block: 0,
            off: 0,
            size: 0,
            buffer: NonNull::from_ref(&[]),
        }
    }
}
