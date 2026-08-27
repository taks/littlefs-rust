//! File handle. Per lfs.h lfs_file_t.

use core::ptr::NonNull;

use crate::bd::LfsCache;
use crate::dir::{LfsMdir, LfsMlist};
use crate::lfs_info::LfsFileConfig;
use crate::lfs_type::{LsfType, OpenFlags};
use crate::types::{lfs_block_t, lfs_off_t};

use super::lfs_ctz::LfsCtz;

/// Per lfs.h typedef struct lfs_file
#[repr(C)]
pub struct LfsFile<'a> {
    pub next: *mut LfsFile<'a>,
    pub id: u16,
    pub type_: LsfType,
    pub m: LfsMdir,
    pub ctz: LfsCtz,
    pub flags: OpenFlags,
    pub pos: lfs_off_t,
    pub block: lfs_block_t,
    pub off: lfs_off_t,
    pub cache: LfsCache,
    pub cfg: NonNull<LfsFileConfig<'a>>,
}

impl Default for LfsFile<'_> {
    fn default() -> Self {
        Self {
            next: Default::default(),
            id: Default::default(),
            type_: Default::default(),
            m: Default::default(),
            ctz: Default::default(),
            flags: Default::default(),
            pos: Default::default(),
            block: Default::default(),
            off: Default::default(),
            cache: Default::default(),
            cfg: NonNull::dangling(),
        }
    }
}

impl<'a> LfsFile<'a> {
    pub(crate) unsafe fn as_mut_lsf_mist(&mut self) -> &mut LfsMlist {
        unsafe { ::core::mem::transmute::<&mut LfsFile<'_>, &mut LfsMlist>(self) }
    }
}
