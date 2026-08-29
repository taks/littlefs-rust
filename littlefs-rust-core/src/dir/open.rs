//! Directory open/read. Per lfs.c lfs_dir_open_, lfs_dir_close_, lfs_dir_read_, etc.

use core::cmp;

use zerocopy::IntoBytes;

use crate::dir::LfsDir;
use crate::dir::fetch::{lfs_dir_fetch, lfs_dir_getinfo};
use crate::dir::find::lfs_dir_find;
use crate::dir::lfs_mlist::lfs_mlist_append;
use crate::dir::lfs_mlist::lfs_mlist_remove;
use crate::dir::traverse::lfs_dir_get;
use crate::error::Error;
use crate::lfs_info::LfsInfo;
use crate::lfs_type::lfs_type::{LFS_TYPE_DIR, LFS_TYPE3_DIR};
use crate::tag::{lfs_mktag, lfs_tag_id, lfs_tag_type3};
use crate::types::lfs_off_t;
use crate::util::{lfs_pair_cmp, lfs_pair_fromle32};

/// Per lfs.c lfs_dir_open_ (lines 2721-2763)
///
/// C:
/// ```c
/// static int lfs_dir_open_(lfs_t *lfs, lfs_dir_t *dir, const char *path) {
///     lfs_stag_t tag = lfs_dir_find(lfs, &dir->m, &path, NULL);
///     if (tag < 0) {
///         return tag;
///     }
///
///     if (lfs_tag_type3(tag) != LFS_TYPE_DIR) {
///         return LFS_ERR_NOTDIR;
///     }
///
///     lfs_block_t pair[2];
///     if (lfs_tag_id(tag) == 0x3ff) {
///         // handle root dir separately
///         pair[0] = lfs->root[0];
///         pair[1] = lfs->root[1];
///     } else {
///         // get dir pair from parent
///         lfs_stag_t res = lfs_dir_get(lfs, &dir->m, LFS_MKTAG(0x700, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_STRUCT, lfs_tag_id(tag), 8), pair);
///         if (res < 0) {
///             return res;
///         }
///         lfs_pair_fromle32(pair);
///     }
///
///     // fetch first pair
///     int err = lfs_dir_fetch(lfs, &dir->m, pair);
///     if (err) {
///         return err;
///     }
///
///     // setup entry
///     dir->head[0] = dir->m.pair[0];
///     dir->head[1] = dir->m.pair[1];
///     dir->id = 0;
///     dir->pos = 0;
///
///     // add to list of mdirs
///     dir->type = LFS_TYPE_DIR;
///     lfs_mlist_append(lfs, (struct lfs_mlist *)dir);
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_open_(lfs: &mut crate::fs::Lfs, dir: &mut LfsDir, path: &str) -> Result<(), Error> {
    let mut path_ptr = path;

    let tag = lfs_dir_find(lfs, &mut dir.m, &mut path_ptr, &mut None)?;

    if (lfs_tag_type3(tag)) != LFS_TYPE3_DIR {
        return Err(Error::NotDir);
    }

    let mut pair = [0u32; 2];
    if lfs_tag_id(tag) == 0x3ff {
        pair[0] = lfs.root[0];
        pair[1] = lfs.root[1];
    } else {
        let _res = lfs_dir_get(
            lfs,
            &dir.m,
            lfs_mktag(0x700, 0x3ff, 0),
            lfs_mktag(
                crate::lfs_type::lfs_type::LFS_TYPE_STRUCT,
                lfs_tag_id(tag) as u32,
                8,
            ),
            pair.as_mut_bytes(),
        )?;

        lfs_pair_fromle32(&mut pair);
    }

    lfs_dir_fetch(lfs, &mut dir.m, pair)?;

    dir.head[0] = dir.m.pair[0];
    dir.head[1] = dir.m.pair[1];
    dir.id = 0;
    dir.pos = 0;
    dir.type_ = LFS_TYPE_DIR as u8;
    lfs_mlist_append(lfs, unsafe { dir.as_mut_lsf_mist() });

    Ok(())
}

/// Per lfs.c lfs_dir_close_ (lines 2765-2770)
///
/// C:
/// ```c
/// static int lfs_dir_close_(lfs_t *lfs, lfs_dir_t *dir) {
///     // remove from list of mdirs
///     lfs_mlist_remove(lfs, (struct lfs_mlist *)dir);
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_close_(lfs: &mut crate::fs::Lfs, dir: &mut LfsDir) -> Result<(), Error> {
    lfs_mlist_remove(lfs, unsafe { dir.as_mut_lsf_mist() });

    Ok(())
}

/// Per lfs.c lfs_dir_read_ (lines 2772-2815)
///
/// C:
/// ```c
/// static int lfs_dir_read_(lfs_t *lfs, lfs_dir_t *dir, struct lfs_info *info) {
///     memset(info, 0, sizeof(*info));
///
///     // special offset for '.' and '..'
///     if (dir->pos == 0) {
///         info->type = LFS_TYPE_DIR;
///         strcpy(info->name, ".");
///         dir->pos += 1;
///         return true;
///     } else if (dir->pos == 1) {
///         info->type = LFS_TYPE_DIR;
///         strcpy(info->name, "..");
///         dir->pos += 1;
///         return true;
///     }
///
///     while (true) {
///         if (dir->id == dir->m.count) {
///             if (!dir->m.split) {
///                 return false;
///             }
///
///             int err = lfs_dir_fetch(lfs, &dir->m, dir->m.tail);
///             if (err) {
///                 return err;
///             }
///
///             dir->id = 0;
///         }
///
///         int err = lfs_dir_getinfo(lfs, &dir->m, dir->id, info);
///         if (err && err != LFS_ERR_NOENT) {
///             return err;
///         }
///
///         dir->id += 1;
///         if (err != LFS_ERR_NOENT) {
///             break;
///         }
///     }
///
///     dir->pos += 1;
///     return true;
/// }
/// ```
pub fn lfs_dir_read_(
    lfs: &mut crate::fs::Lfs,
    dir: &mut LfsDir,
    info: &mut LfsInfo,
) -> Result<bool, Error> {
    {
        info.type_ = 0;
        info.size = 0;
        info.name.fill(0);

        if dir.pos == 0 {
            info.type_ = LFS_TYPE_DIR as u8;
            info.name[0] = b'.';
            info.name[1] = 0;
            dir.pos += 1;
            return Ok(true);
        }
        if dir.pos == 1 {
            info.type_ = LFS_TYPE_DIR as u8;
            info.name[0] = b'.';
            info.name[1] = b'.';
            info.name[2] = 0;
            dir.pos += 1;
            return Ok(true);
        }

        loop {
            if dir.id == dir.m.count {
                if !dir.m.split {
                    return Ok(false);
                }
                let dir_m_tail = dir.m.tail;
                lfs_dir_fetch(lfs, &mut dir.m, dir_m_tail)?;

                dir.id = 0;
            }

            let err = lfs_dir_getinfo(lfs, &dir.m, dir.id, info);
            if let Err(err) = err
                && err != Error::NoEntry
            {
                return crate::lfs_pass_err!(Err(err));
            }
            dir.id += 1;
            if err != Err(Error::NoEntry) {
                break;
            }
        }

        dir.pos += 1;
        Ok(true)
    }
}

/// Per lfs.c lfs_dir_seek_ (lines 2817-2851)
///
/// C:
/// ```c
/// static int lfs_dir_seek_(lfs_t *lfs, lfs_dir_t *dir, lfs_off_t off) {
///     // simply walk from head dir
///     int err = lfs_dir_rewind_(lfs, dir);
///     if (err) {
///         return err;
///     }
///
///     // first two for ./..
///     dir->pos = lfs_min(2, off);
///     off -= dir->pos;
///
///     // skip superblock entry
///     dir->id = (off > 0 && lfs_pair_cmp(dir->head, lfs->root) == 0);
///
///     while (off > 0) {
///         if (dir->id == dir->m.count) {
///             if (!dir->m.split) {
///                 return LFS_ERR_INVAL;
///             }
///
///             err = lfs_dir_fetch(lfs, &dir->m, dir->m.tail);
///             if (err) {
///                 return err;
///             }
///
///             dir->id = 0;
///         }
///
///         int diff = lfs_min(dir->m.count - dir->id, off);
///         dir->id += diff;
///         dir->pos += diff;
///         off -= diff;
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_dir_seek_(
    lfs: &mut crate::fs::Lfs,
    dir: &mut LfsDir,
    off: lfs_off_t,
) -> Result<(), Error> {
    lfs_dir_rewind_(lfs, dir)?;

    dir.pos = cmp::min(2, off);
    let mut off = off - dir.pos;

    // skip superblock entry
    dir.id = if off > 0 && !lfs_pair_cmp(&dir.head, &lfs.root) {
        1
    } else {
        0
    };

    while off > 0 {
        if dir.id == dir.m.count {
            if !dir.m.split {
                return Err(Error::Invalid);
            }
            let dir_m_tail = dir.m.tail;
            lfs_dir_fetch(lfs, &mut dir.m, dir_m_tail)?;
            dir.id = 0;
        }
        let diff = cmp::min((dir.m.count - dir.id) as u32, off);
        dir.id += diff as u16;
        dir.pos += diff;
        off -= diff;
    }

    Ok(())
}

/// Per lfs.c lfs_dir_tell_ (lines 2854-2857)
///
/// C:
/// ```c
/// static lfs_soff_t lfs_dir_tell_(lfs_t *lfs, lfs_dir_t *dir) {
///     (void)lfs;
///     return dir->pos;
/// }
/// ```
pub fn lfs_dir_tell_(_lfs: *mut crate::fs::Lfs, dir: *const LfsDir) -> crate::types::lfs_soff_t {
    unsafe { (*dir).pos as crate::types::lfs_soff_t }
}

/// Per lfs.c lfs_dir_rewind_ (lines 2859-2869)
///
/// C:
/// ```c
/// static int lfs_dir_rewind_(lfs_t *lfs, lfs_dir_t *dir) {
///     // reload the head dir
///     int err = lfs_dir_fetch(lfs, &dir->m, dir->head);
///     if (err) {
///         return err;
///     }
///
///     dir->id = 0;
///     dir->pos = 0;
///     return 0;
/// }
/// ```
pub fn lfs_dir_rewind_(lfs: &mut crate::fs::Lfs, dir: &mut LfsDir) -> Result<(), Error> {
    lfs_dir_fetch(lfs, &mut dir.m, dir.head)?;
    dir.id = 0;
    dir.pos = 0;
    Ok(())
}
