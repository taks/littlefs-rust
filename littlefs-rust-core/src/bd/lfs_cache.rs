//! Block cache. Per lfs.h lfs_cache_t.

use core::{fmt::Debug, ptr::NonNull};

use crate::types::lfs_block_t;

/// Per lfs.h typedef struct lfs_cache
pub struct LfsCache {
    pub block: lfs_block_t,
    pub off: usize,
    pub size: usize,
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

impl Debug for LfsCache {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LfsCache")
            .field("block", &self.block)
            .field("off", &self.off)
            .field("size", &self.size)
            .field("buffer", &self.buffer)
            .finish()
    }
}
