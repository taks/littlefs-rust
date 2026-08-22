//! CTZ struct (file block list). Per lfs.h lfs_file_t.ctz.

use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::types::{lfs_block_t, lfs_size_t};
use crate::util::lfs_tole32;

/// Per lfs.h struct lfs_ctz (in lfs_file_t)
#[repr(C)]
#[derive(Clone, Copy, Default, FromBytes, IntoBytes, Immutable)]
pub struct LfsCtz {
    pub head: lfs_block_t,
    pub size: lfs_size_t,
}

/// Per lfs.c lfs_ctz_fromle32
#[inline(always)]
pub fn lfs_ctz_fromle32(ctz: &mut LfsCtz) {
    ctz.head = u32::from_le(ctz.head);
    ctz.size = u32::from_le(ctz.size);
}

/// Per lfs.c lfs_ctz_tole32
#[inline(always)]
pub fn lfs_ctz_tole32(ctz: &mut LfsCtz) {
    ctz.head = lfs_tole32(ctz.head);
    ctz.size = lfs_tole32(ctz.size);
}
