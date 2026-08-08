//! Stat. Per lfs.c lfs_stat_, lfs_fs_stat_, lfs_fs_size_.

use core::ffi::CStr;
use zerocopy::IntoBytes;

use crate::borrow_unchecked::borrow_unchecked;
use crate::error::Error;
use crate::fs::traverse::lfs_fs_traverse_;
use crate::types::{lfs_block_t, lfs_size_t};

/// Per lfs.c lfs_stat_ (lines 3863-3878)
///
/// C:
/// ```c
/// static int lfs_stat_(lfs_t *lfs, const char *path, struct lfs_info *info) {
///     lfs_mdir_t cwd;
///     lfs_stag_t tag = lfs_dir_find(lfs, &cwd, &path, NULL);
///     if (tag < 0) {
///         return (int)tag;
///     }
///
///     // only allow trailing slashes on dirs
///     if (strchr(path, '/') != NULL
///             && lfs_tag_type3(tag) != LFS_TYPE_DIR) {
///         return LFS_ERR_NOTDIR;
///     }
///
///     return lfs_dir_getinfo(lfs, &cwd, lfs_tag_id(tag), info);
/// }
/// ```
pub fn lfs_stat_(
    lfs: &mut super::lfs::Lfs,
    path: &CStr,
    info: &mut crate::lfs_info::LfsInfo,
) -> Result<(), Error> {
    use crate::dir::fetch::lfs_dir_getinfo;
    use crate::dir::find::lfs_dir_find;
    use crate::lfs_type::lfs_type::LFS_TYPE_DIR;
    use crate::tag::{lfs_tag_id, lfs_tag_type3};

    unsafe {
        let mut cwd = core::mem::zeroed::<crate::dir::LfsMdir>();
        let mut path_ptr = path;

        let tag = lfs_dir_find(lfs, &mut cwd, &mut path_ptr, &mut None)?;

        // C: lfs.c:3872-3875 - only allow trailing slashes on dirs (strchr(path, '/') != NULL)
        let mut p = path_ptr.as_ptr() as *const u8;
        #[cfg(feature = "loop_limits")]
        const MAX_STAT_PATH_ITER: u32 = 1024;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;
        while *p != 0 {
            #[cfg(feature = "loop_limits")]
            {
                if iter >= MAX_STAT_PATH_ITER {
                    panic!(
                        "loop_limits: MAX_STAT_PATH_ITER ({}) exceeded",
                        MAX_STAT_PATH_ITER
                    );
                }
                iter += 1;
            }
            if *p == b'/' {
                if u32::from(lfs_tag_type3(tag)) != LFS_TYPE_DIR {
                    return Err(Error::NotDir);
                }
                break;
            }
            p = p.add(1);
        }

        lfs_dir_getinfo(lfs, &cwd, lfs_tag_id(tag as u32), info)
    }
}

/// Per lfs.c lfs_fs_stat_ (lines 4653-4691)
///
/// C:
/// ```c
/// static int lfs_fs_stat_(lfs_t *lfs, struct lfs_fsinfo *fsinfo) {
///     // if the superblock is up-to-date, we must be on the most recent
///     // minor version of littlefs
///     if (!lfs_gstate_needssuperblock(&lfs->gstate)) {
///         fsinfo->disk_version = lfs_fs_disk_version(lfs);
///
///     // otherwise we need to read the minor version on disk
///     } else {
///         // fetch the superblock
///         lfs_mdir_t dir;
///         int err = lfs_dir_fetch(lfs, &dir, lfs->root);
///         if (err) {
///             return err;
///         }
///
///         lfs_superblock_t superblock;
///         lfs_stag_t tag = lfs_dir_get(lfs, &dir, LFS_MKTAG(0x7ff, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///                 &superblock);
///         if (tag < 0) {
///             return tag;
///         }
///         lfs_superblock_fromle32(&superblock);
///
///         // read the on-disk version
///         fsinfo->disk_version = superblock.version;
///     }
///
///     // filesystem geometry
///     fsinfo->block_size = lfs->cfg->block_size;
///     fsinfo->block_count = lfs->block_count;
///
///     // other on-disk configuration, we cache all of these for internal use
///     fsinfo->name_max = lfs->name_max;
///     fsinfo->file_max = lfs->file_max;
///     fsinfo->attr_max = lfs->attr_max;
///
///     return 0;
/// }
/// ```
pub fn lfs_fs_stat_(
    lfs: &mut super::lfs::Lfs,
    fsinfo: *mut crate::lfs_info::LfsFsinfo,
) -> Result<(), Error> {
    use crate::dir::fetch::lfs_dir_fetch;
    use crate::dir::traverse::lfs_dir_get;
    use crate::lfs_gstate::lfs_gstate_needssuperblock;
    use crate::lfs_superblock::{LfsSuperblock, lfs_superblock_fromle32};
    use crate::lfs_type::lfs_type::LFS_TYPE_INLINESTRUCT;
    use crate::tag::lfs_mktag;
    use crate::types::LFS_DISK_VERSION;

    unsafe {
        let fsinfo = &mut *fsinfo;

        if !lfs_gstate_needssuperblock(&lfs.gstate) {
            fsinfo.disk_version = LFS_DISK_VERSION;
        } else {
            let mut dir = crate::dir::LfsMdir {
                pair: [0, 0],
                rev: 0,
                off: 0,
                etag: 0,
                count: 0,
                erased: false,
                split: false,
                tail: [0, 0],
            };
            let lfs_root = borrow_unchecked(&lfs.root);
            lfs_dir_fetch(lfs, &mut dir, lfs_root)?;

            let mut superblock = core::mem::zeroed::<LfsSuperblock>();
            let tag = lfs_dir_get(
                lfs,
                &dir,
                lfs_mktag(0x7ff, 0x3ff, 0),
                lfs_mktag(
                    LFS_TYPE_INLINESTRUCT,
                    0,
                    core::mem::size_of::<LfsSuperblock>() as u32,
                ),
                superblock.as_mut_bytes(),
            )?;

            lfs_superblock_fromle32(&mut superblock);
            fsinfo.disk_version = superblock.version;
        }

        fsinfo.block_size = (*lfs.cfg).block_size;
        fsinfo.block_count = lfs.block_count;
        fsinfo.name_max = lfs.name_max;
        fsinfo.file_max = lfs.file_max;
        fsinfo.attr_max = lfs.attr_max;
    }
    Ok(())
}

/// Per lfs.c lfs_fs_size_count (lines 5172-5177)
///
/// C:
/// ```c
/// static int lfs_fs_size_count(void *p, lfs_block_t block) {
///     (void)block;
///     lfs_size_t *size = p;
///     *size += 1;
///     return 0;
/// }
/// ```
pub fn lfs_fs_size_count(p: *mut core::ffi::c_void, _block: lfs_block_t) -> Result<(), Error> {
    if p.is_null() {
        return Ok(());
    }
    let size = p as *mut lfs_size_t;
    unsafe { *size = (*size).saturating_add(1) };
    Ok(())
}

/// Per lfs.c lfs_fs_size_ (lines 5179-5188)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_fs_size_(lfs_t *lfs) {
///     lfs_size_t size = 0;
///     int err = lfs_fs_traverse_(lfs, lfs_fs_size_count, &size, false);
///     if (err) {
///         return err;
///     }
///
///     return size;
/// }
/// ```
pub fn lfs_fs_size_(lfs: &mut super::lfs::Lfs) -> Result<lfs_size_t, Error> {
    let mut size: lfs_size_t = 0;
    lfs_fs_traverse_(
        lfs,
        Some(lfs_fs_size_count),
        &mut size as *mut _ as *mut core::ffi::c_void,
        false,
    )?;

    Ok(size)
}
