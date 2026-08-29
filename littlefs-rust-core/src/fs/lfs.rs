//! Main filesystem type. Per lfs.h typedef struct lfs.

use core::cell::{RefCell, UnsafeCell};
use core::fmt::Debug;
use core::ptr::NonNull;

use crate::bd::LfsCache;
use crate::dir::LfsMlist;
use crate::lfs_config::LfsConfig;
use crate::lfs_gstate::LfsGstate;
use crate::types::lfs_block_t;

use super::lfs_lookahead::LfsLookahead;

/// Per lfs.h typedef struct lfs
#[repr(C)]
pub struct Lfs<S> {
    pub rcache: UnsafeCell<LfsCache>,
    pub pcache: UnsafeCell<LfsCache>,
    pub root: [lfs_block_t; 2],
    pub mlist: *mut LfsMlist,
    pub seed: u32,
    pub gstate: LfsGstate,
    pub gdisk: LfsGstate,
    pub gdelta: RefCell<LfsGstate>,
    pub lookahead: LfsLookahead,
    pub cfg: NonNull<LfsConfig<S>>,
    pub block_count: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
    pub inline_max: u32,
}

impl Default for Lfs {
    fn default() -> Self {
        Self {
            rcache: Default::default(),
            pcache: Default::default(),
            root: Default::default(),
            mlist: Default::default(),
            seed: Default::default(),
            gstate: Default::default(),
            gdisk: Default::default(),
            gdelta: Default::default(),
            lookahead: Default::default(),
            cfg: NonNull::dangling(),
            block_count: Default::default(),
            name_max: Default::default(),
            file_max: Default::default(),
            attr_max: Default::default(),
            inline_max: Default::default(),
        }
    }
}

impl Debug for Lfs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("Lfs");
        unsafe {
            if let Some(mlist) = self.mlist.as_ref() {
                s.field("mlist", mlist);
            }
        }
        s.finish()
    }
}
