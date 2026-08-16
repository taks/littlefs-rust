//! File handle. Per lfs.h lfs_file_t.

use crate::bd::LfsCache;
use crate::dir::{LfsMdir, LfsMlist};
use crate::lfs_info::LfsFileConfig;
use crate::lfs_type::LsfType;
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
    pub flags: u32,
    pub pos: lfs_off_t,
    pub block: lfs_block_t,
    pub off: lfs_off_t,
    pub cache: LfsCache,
    pub cfg: *const LfsFileConfig<'a, ()>,
}

impl<'a> LfsFile<'a> {
    pub(crate) unsafe fn as_mut_lsf_mist(&mut self) -> *mut LfsMlist {
        unsafe {
            ::core::mem::transmute::<*mut LfsFile<'_>, *mut LfsMlist>(::core::ptr::from_mut(self))
        }
    }
}
