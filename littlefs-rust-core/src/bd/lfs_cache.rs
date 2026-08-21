//! Block cache. Per lfs.h lfs_cache_t.

use core::ptr::NonNull;

use crate::types::lfs_block_t;

/// Per lfs.h typedef struct lfs_cache
#[derive(Default)]
pub struct LfsCache {
    pub block: lfs_block_t,
    pub off: u32,
    pub size: u32,
    pub buffer: Option<NonNull<[u8]>>,
}
