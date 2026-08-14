//! Directory handle. Per lfs.h lfs_dir_t.

use super::lfs_mdir::LfsMdir;
use crate::{
    dir::LfsMlist,
    types::{lfs_block_t, lfs_off_t},
};

/// Per lfs.h typedef struct lfs_dir
#[repr(C)]
pub struct LfsDir {
    pub next: *mut LfsDir,
    pub id: u16,
    pub type_: u8,
    pub m: LfsMdir,
    pub pos: lfs_off_t,
    pub head: [lfs_block_t; 2],
}

impl LfsDir {
    pub(crate) unsafe fn as_mut_lsf_mist(&mut self) -> *mut LfsMlist {
        unsafe { ::core::mem::transmute::<*mut Self, *mut LfsMlist>(::core::ptr::from_mut(self)) }
    }
}
