//! FS grow/shrink. Per lfs.c lfs_fs_grow_, lfs_shrink_checkblock.

use zerocopy::IntoBytes;

use crate::borrow_unchecked::borrow_unchecked;
use crate::dir::LfsMdir;
use crate::dir::commit::lfs_dir_commit;
use crate::dir::fetch::lfs_dir_fetch;
use crate::dir::traverse::lfs_dir_get;
use crate::error::Error;
use crate::lfs_superblock::{LfsSuperblock, lfs_superblock_fromle32, lfs_superblock_tole32};
use crate::lfs_type::lfs_type::LFS_TYPE_INLINESTRUCT;
use crate::tag::{lfs_mattr, lfs_mktag};
use crate::types::{lfs_block_t, lfs_size_t};

/// Translation docs: Callback for lfs_fs_traverse_ during shrink. Returns
/// LFS_ERR_NOTEMPTY if any in-use block is at or beyond the target threshold,
/// preventing a shrink that would lose data.
///
/// C: lfs.c:5244-5251
/// ```c
/// static int lfs_shrink_checkblock(void *data, lfs_block_t block) {
///     lfs_size_t threshold = *((lfs_size_t*)data);
///     if (block >= threshold) {
///         return LFS_ERR_NOTEMPTY;
///     }
///     return 0;
/// }
/// ```
fn lfs_shrink_checkblock(data: *mut core::ffi::c_void, block: lfs_block_t) -> Result<(), Error> {
    let threshold = unsafe { *(data as *const lfs_size_t) };
    if block >= threshold {
        return Err(Error::NotEmpty);
    }
    Ok(())
}

/// Translation docs: Grow or shrink the filesystem to a new block_count.
/// If shrinking, traverses all blocks to verify none above the new count
/// are in use. Updates the superblock's block_count on disk.
///
/// C: lfs.c:5253-5303
/// ```c
/// static int lfs_fs_grow_(lfs_t *lfs, lfs_size_t block_count) {
///     int err;
///
///     if (block_count == lfs->block_count) {
///         return 0;
///     }
///
/// #ifndef LFS_SHRINKNONRELOCATING
///     // shrinking is not supported
///     LFS_ASSERT(block_count >= lfs->block_count);
/// #endif
/// #ifdef LFS_SHRINKNONRELOCATING
///     if (block_count < lfs->block_count) {
///         err = lfs_fs_traverse_(lfs, lfs_shrink_checkblock,
///                 &block_count, true);
///         if (err) {
///             return err;
///         }
///     }
/// #endif
///
///     lfs->block_count = block_count;
///
///     // fetch the root
///     lfs_mdir_t root;
///     err = lfs_dir_fetch(lfs, &root, lfs->root);
///     if (err) {
///         return err;
///     }
///
///     // update the superblock
///     lfs_superblock_t superblock;
///     lfs_stag_t tag = lfs_dir_get(lfs, &root,
///             LFS_MKTAG(0x7ff, 0x3ff, 0),
///             LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///             &superblock);
///     if (tag < 0) {
///         return tag;
///     }
///     lfs_superblock_fromle32(&superblock);
///
///     superblock.block_count = lfs->block_count;
///
///     lfs_superblock_tole32(&superblock);
///     err = lfs_dir_commit(lfs, &root, LFS_MKATTRS(
///             {tag, &superblock}));
///     if (err) {
///         return err;
///     }
///     return 0;
/// }
/// ```
pub fn lfs_fs_grow_(lfs: &mut super::lfs::Lfs, block_count: lfs_size_t) -> Result<(), Error> {
    unsafe {
        if block_count == lfs.block_count {
            return Ok(());
        }

        // LFS_SHRINKNONRELOCATING path: check no blocks above threshold in use
        if block_count < lfs.block_count {
            let mut threshold = block_count;
            super::traverse::lfs_fs_traverse_(
                lfs,
                lfs_shrink_checkblock,
                &mut threshold as *mut _ as *mut core::ffi::c_void,
                true,
            )?;
        }

        lfs.block_count = block_count;

        // fetch the root
        let mut root = core::mem::MaybeUninit::<LfsMdir>::zeroed();
        let root = root.assume_init_mut();
        let lfs_root = borrow_unchecked(&lfs.root);
        lfs_dir_fetch(lfs, root, lfs_root)?;

        // update the superblock
        let mut superblock = core::mem::zeroed::<LfsSuperblock>();
        let tag = lfs_dir_get(
            lfs,
            root,
            lfs_mktag(0x7ff, 0x3ff, 0),
            lfs_mktag(
                LFS_TYPE_INLINESTRUCT,
                0,
                core::mem::size_of::<LfsSuperblock>() as u32,
            ),
            superblock.as_mut_bytes(),
        )?;

        lfs_superblock_fromle32(&mut superblock);
        superblock.block_count = lfs.block_count;

        lfs_superblock_tole32(&mut superblock);
        // C: lfs_dir_commit(lfs, &root, LFS_MKATTRS({tag, &superblock}))
        let attrs = [lfs_mattr {
            tag: tag as u32,
            buffer: superblock.as_bytes(),
        }];
        lfs_dir_commit(lfs, root, &attrs)?;

        Ok(())
    }
}
