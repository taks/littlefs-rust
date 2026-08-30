//! FS parent. Per lfs.c lfs_fs_pred, lfs_fs_parent.

use zerocopy::IntoBytes;

use crate::{Storage, error::Error, lfs_pass_err};

/// Per lfs.c lfs_fs_pred (lines 4796-4833)
///
/// C:
/// ```c
/// static int lfs_fs_pred(lfs_t *lfs,
///         const lfs_block_t pair[2], lfs_mdir_t *pdir) {
///     // iterate over all directory directory entries
///     pdir->tail[0] = 0;
///     pdir->tail[1] = 1;
///     struct lfs_tortoise_t tortoise = {
///         .pair = {LFS_BLOCK_NULL, LFS_BLOCK_NULL},
///         .i = 1,
///         .period = 1,
///     };
///     int err = LFS_ERR_OK;
///     while (!lfs_pair_isnull(pdir->tail)) {
///         err = lfs_tortoise_detectcycles(pdir, &tortoise);
///         if (err < 0) {
///             return LFS_ERR_CORRUPT;
///         }
///
///         if (lfs_pair_cmp(pdir->tail, pair) == 0) {
///             return 0;
///         }
///
///         int err = lfs_dir_fetch(lfs, pdir, pdir->tail);
///         if (err) {
///             return err;
///         }
///     }
///
///     return LFS_ERR_NOENT;
/// }
/// #endif
/// ```
pub fn lfs_fs_pred<S>(
    lfs: &mut crate::fs::Lfs<S>,
    pair: &[crate::types::lfs_block_t; 2],
    pdir: &mut crate::dir::LfsMdir,
) -> Result<(), Error> {
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::fs::mount::{LfsTortoise, lfs_tortoise_detectcycles};
    use crate::types::LFS_BLOCK_NULL;
    use crate::util::{lfs_pair_cmp, lfs_pair_isnull};

    pdir.tail = [0, 1];
    let mut tortoise = LfsTortoise {
        pair: [LFS_BLOCK_NULL, LFS_BLOCK_NULL],
        i: 1,
        period: 1,
    };
    let mut have_fetched = false;

    while !lfs_pair_isnull(&pdir.tail) {
        let err = lfs_tortoise_detectcycles(pdir, &mut tortoise);
        if err.is_err() {
            return Err(Error::Corrupt);
        }

        if !lfs_pair_cmp(&pdir.tail, pair) {
            if !have_fetched {
                // Matched before any fetch: tail [0,1] == pair (root).
                // The root has no predecessor.
                lfs_dir_fetch(lfs, pdir, pdir.tail)?;

                if lfs_pair_isnull(&pdir.tail) {
                    return Err(crate::error::Error::NoEntry);
                }
            }
            return Ok(());
        }

        lfs_dir_fetch(lfs, pdir, pdir.tail)?;
        have_fetched = true;
    }

    Err(Error::NoEntry)
}

/// C: lfs.c:4835-4853
#[repr(C)]
pub struct LfsFsParentMatch<S> {
    pub lfs: *mut crate::fs::Lfs<S>,
    pub pair: [crate::types::lfs_block_t; 2],
}

/// Per lfs.c lfs_fs_parent_match (lines 4835-4853)
///
/// C:
/// ```c
/// static int lfs_fs_parent_match(void *data,
///         lfs_tag_t tag, const void *buffer) {
///     struct lfs_fs_parent_match *find = data;
///     lfs_t *lfs = find->lfs;
///     const struct lfs_diskoff *disk = buffer;
///     (void)tag;
///     lfs_block_t child[2];
///     int err = lfs_bd_read(lfs, ...);
///     lfs_pair_fromle32(child);
///     return (lfs_pair_cmp(child, find->pair) == 0) ? LFS_CMP_EQ : LFS_CMP_LT;
/// }
/// ```
pub async fn lfs_fs_parent_match<S: Storage>(
    find: &LfsFsParentMatch<S>,
    disk: &crate::tag::lfs_diskoff,
) -> Result<core::cmp::Ordering, Error> {
    use crate::bd::bd::lfs_bd_read;
    use crate::util::{lfs_pair_cmp, lfs_pair_fromle32};

    let mut child: [crate::types::lfs_block_t; 2] = [0, 0];
    let lfs = unsafe { find.lfs.as_ref().unwrap() };
    lfs_bd_read(
        lfs,
        None,
        unsafe { &mut *lfs.rcache.get() },
        unsafe { lfs.cfg.as_ref().block_size as usize },
        disk.block,
        disk.off as usize,
        child.as_mut_bytes(),
    ).await?;

    lfs_pair_fromle32(&mut child);
    if !lfs_pair_cmp(&child, &find.pair) {
        Ok(core::cmp::Ordering::Equal)
    } else {
        Ok(core::cmp::Ordering::Less)
    }
}

/// Per lfs.c lfs_fs_parent (lines 4856-4892)
///
/// C:
/// ```c
/// static lfs_stag_t lfs_fs_parent(lfs_t *lfs, const lfs_block_t pair[2],
///         lfs_mdir_t *parent) {
///     // use fetchmatch with callback to find pairs
///     parent->tail[0] = 0;
///     parent->tail[1] = 1;
///     struct lfs_tortoise_t tortoise = {
///         .pair = {LFS_BLOCK_NULL, LFS_BLOCK_NULL},
///         .i = 1,
///         .period = 1,
///     };
///     int err = LFS_ERR_OK;
///     while (!lfs_pair_isnull(parent->tail)) {
///         err = lfs_tortoise_detectcycles(parent, &tortoise);
///         if (err < 0) {
///             return err;
///         }
///
///         lfs_stag_t tag = lfs_dir_fetchmatch(lfs, parent, parent->tail,
///                 LFS_MKTAG(0x7ff, 0, 0x3ff),
///                 LFS_MKTAG(LFS_TYPE_DIRSTRUCT, 0, 8),
///                 NULL,
///                 lfs_fs_parent_match, &(struct lfs_fs_parent_match){
///                     lfs, {pair[0], pair[1]}});
///         if (tag && tag != LFS_ERR_NOENT) {
///             return tag;
///         }
///     }
///
///     return LFS_ERR_NOENT;
/// }
/// #endif
/// ```
pub fn lfs_fs_parent(
    lfs: &mut crate::fs::Lfs,
    pair: &[crate::types::lfs_block_t; 2],
    parent: &mut crate::dir::LfsMdir,
) -> Result<crate::types::lfs_tag_t, Error> {
    use crate::dir::fetch::lfs_dir_fetchmatch;
    use crate::fs::mount::{LfsTortoise, lfs_tortoise_detectcycles};
    use crate::lfs_type::lfs_type::LFS_TYPE_DIRSTRUCT;
    use crate::tag::lfs_mktag;
    use crate::types::LFS_BLOCK_NULL;
    use crate::util::lfs_pair_isnull;

    parent.tail = [0, 1];
    let mut tortoise = LfsTortoise {
        pair: [LFS_BLOCK_NULL, LFS_BLOCK_NULL],
        i: 1,
        period: 1,
    };

    while !lfs_pair_isnull(&parent.tail) {
        lfs_pass_err!(lfs_tortoise_detectcycles(parent, &mut tortoise))?;

        let find_match = LfsFsParentMatch {
            lfs,
            pair: [(*pair)[0], (*pair)[1]],
        };
        let tag = lfs_dir_fetchmatch(
            lfs,
            parent,
            parent.tail,
            lfs_mktag(0x7ff, 0, 0x3ff),
            lfs_mktag(LFS_TYPE_DIRSTRUCT, 0, 8),
            &mut None,
            Some(&|_, disk| lfs_fs_parent_match(&find_match, disk)),
        );

        if tag != Ok(0) && tag != Err(Error::NoEntry) {
            return tag;
        }
    }

    Err(Error::NoEntry)
}
