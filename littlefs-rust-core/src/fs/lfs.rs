//! Main filesystem type. Per lfs.h typedef struct lfs.

use core::cell::{RefCell, UnsafeCell};

use crate::bd::LfsCache;
use crate::dir::LfsMlist;
use crate::lfs_config::LfsConfig;
use crate::lfs_gstate::LfsGstate;
use crate::types::lfs_block_t;

use super::lfs_lookahead::LfsLookahead;

/// Per lfs.h typedef struct lfs
#[repr(C)]
#[derive(Default)]
pub struct Lfs {
    pub rcache: UnsafeCell<LfsCache>,
    pub pcache: UnsafeCell<LfsCache>,
    pub root: [lfs_block_t; 2],
    pub mlist: *mut LfsMlist,
    pub seed: u32,
    pub gstate: LfsGstate,
    pub gdisk: LfsGstate,
    pub gdelta: RefCell<LfsGstate>,
    pub lookahead: LfsLookahead,
    pub cfg: *const LfsConfig,
    pub block_count: u32,
    pub name_max: u32,
    pub file_max: u32,
    pub attr_max: u32,
    pub inline_max: u32,
}
