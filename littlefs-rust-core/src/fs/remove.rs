//! remove. Per lfs.c remove_.

use core::ffi::CStr;
use zerocopy::IntoBytes;

use crate::dir::commit::{lfs_dir_commit, lfs_dir_drop};
use crate::dir::fetch::lfs_dir_fetch;
use crate::dir::find::lfs_dir_find;
use crate::dir::traverse::lfs_dir_get;
use crate::dir::{LfsMdir, LfsMlist};
use crate::error::Error;
use crate::fs::parent::lfs_fs_pred;
use crate::fs::superblock::{lfs_fs_forceconsistency, lfs_fs_preporphans};
use crate::lfs_gstate::lfs_gstate_hasorphans;
use crate::lfs_type::lfs_type::{LFS_TYPE_DELETE, LFS_TYPE_DIR, LFS_TYPE_STRUCT};
use crate::tag::{Tag::mktag, lfs_mattr, lfs_tag_id, lfs_tag_type3};
use crate::types::lfs_block_t;
use crate::util::lfs_pair_fromle32;

/// Per lfs.c lfs_remove_ (lines 3880-3960)
///
/// C:
/// ```c
/// static int lfs_remove_(lfs_t *lfs, const char *path) {
///     // deorphan if we haven't yet, needed at most once after poweron
///     int err = lfs_fs_forceconsistency(lfs);
///     if (err) {
///         return err;
///     }
///
///     lfs_mdir_t cwd;
///     lfs_stag_t tag = lfs_dir_find(lfs, &cwd, &path, NULL);
///     if (tag < 0 || lfs_tag_id(tag) == 0x3ff) {
///         return (tag < 0) ? (int)tag : LFS_ERR_INVAL;
///     }
///
///     struct lfs_mlist dir;
///     dir.next = lfs->mlist;
///     if (lfs_tag_type3(tag) == LFS_TYPE_DIR) {
///         // must be empty before removal
///         lfs_block_t pair[2];
///         lfs_stag_t res = lfs_dir_get(lfs, &cwd, LFS_MKTAG(0x700, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_STRUCT, lfs_tag_id(tag), 8), pair);
///         if (res < 0) {
///             return (int)res;
///         }
///         lfs_pair_fromle32(pair);
///
///         err = lfs_dir_fetch(lfs, &dir.m, pair);
///         if (err) {
///             return err;
///         }
///
///         if (dir.m.count > 0 || dir.m.split) {
///             return LFS_ERR_NOTEMPTY;
///         }
///
///         // mark fs as orphaned
///         err = lfs_fs_preporphans(lfs, +1);
///         if (err) {
///             return err;
///         }
///
///         // I know it's crazy but yes, dir can be changed by our parent's
///         // commit (if predecessor is child)
///         dir.type = 0;
///         dir.id = 0;
///         lfs->mlist = &dir;
///     }
///
///     // delete the entry
///     err = lfs_dir_commit(lfs, &cwd, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_DELETE, lfs_tag_id(tag), 0), NULL}));
///     if (err) {
///         lfs->mlist = dir.next;
///         return err;
///     }
///
///     lfs->mlist = dir.next;
///     if (lfs_gstate_hasorphans(&lfs->gstate)) {
///         LFS_ASSERT(lfs_tag_type3(tag) == LFS_TYPE_DIR);
///
///         // fix orphan
///         err = lfs_fs_preporphans(lfs, -1);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_fs_pred(lfs, dir.m.pair, &cwd);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_dir_drop(lfs, &cwd, &dir.m);
///         if (err) {
///             return err;
///         }
///     }
///
///     return 0;
/// }
/// #endif
///
/// #ifndef LFS_READONLY
/// ```
pub fn lfs_remove_(lfs: &mut super::lfs::Lfs, path: &CStr) -> Result<(), Error> {
    lfs_fs_forceconsistency(lfs)?;

    unsafe {
        let mut cwd = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [(*lfs).root[0], (*lfs).root[1]],
        };

        let mut path_ptr = path;
        let tag = lfs_dir_find(lfs, &mut cwd, &mut path_ptr, &mut None)?;
        if lfs_tag_id(tag as u32) == 0x3ff {
            return Err(Error::Invalid);
        }

        let mut dir = LfsMlist {
            next: (*lfs).mlist,
            id: 0,
            type_: 0,
            m: core::mem::zeroed(),
        };

        if u32::from(lfs_tag_type3(tag as u32)) == LFS_TYPE_DIR {
            let mut pair: [lfs_block_t; 2] = [0, 0];
            let res = lfs_dir_get(
                lfs,
                &cwd,
                Tag::mktag(0x700, 0x3ff, 0),
                Tag::mktag(LFS_TYPE_STRUCT, lfs_tag_id(tag as u32) as u32, 8),
                pair.as_mut_bytes(),
            )?;
            lfs_pair_fromle32(&mut pair);

            lfs_dir_fetch(lfs, &mut dir.m, &pair)?;

            if dir.m.count > 0 || dir.m.split {
                return crate::lfs_err!(Err(Error::NotEmpty));
            }

            lfs_fs_preporphans(lfs, 1)?;

            dir.type_ = 0;
            dir.id = 0;
            (*lfs).mlist = &dir as *const _ as *mut _;
        }

        let attrs = [lfs_mattr {
            tag: Tag::mktag(LFS_TYPE_DELETE, lfs_tag_id(tag as u32) as u32, 0),
            buffer: &[],
        }];
        let err = lfs_dir_commit(lfs, &mut cwd, &attrs);
        (*lfs).mlist = dir.next;
        if err.is_err() {
            return crate::lfs_pass_err!(err);
        }

        if lfs_gstate_hasorphans(&(*lfs).gstate) {
            crate::lfs_assert!(u32::from(lfs_tag_type3(tag as u32)) == LFS_TYPE_DIR);

            lfs_fs_preporphans(lfs, -1)?;

            lfs_fs_pred(lfs, &dir.m.pair, &mut cwd)?;

            lfs_dir_drop(lfs, &mut cwd, &dir.m)
        } else {
            Ok(())
        }
    }
}
