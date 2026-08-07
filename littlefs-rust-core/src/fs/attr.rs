//! attr. Per lfs.c attr_.

use core::ffi::CStr;

use crate::borrow_unchecked::borrow_unchecked;
use crate::dir::LfsMdir;
use crate::dir::commit::lfs_dir_commit;
use crate::dir::fetch::lfs_dir_fetch;
use crate::dir::find::lfs_dir_find;
use crate::dir::traverse::lfs_dir_get;
use crate::error::Error;
use crate::fs::{Lfs, lfs};
use crate::lfs_type::lfs_type::LFS_TYPE_USERATTR;
use crate::tag::{lfs_mattr, lfs_mktag, lfs_tag_id, lfs_tag_size};
use crate::types::lfs_size_t;
use crate::util::lfs_min;

/// Per lfs.c lfs_getattr_ (lines 4107-4135)
///
/// Translation docs: Resolve path, get id (0 for root), lfs_dir_get for USERATTR, return size or error.
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_getattr_(lfs_t *lfs, const char *path,
///         uint8_t type, void *buffer, lfs_size_t size) {
///     lfs_mdir_t cwd;
///     lfs_stag_t tag = lfs_dir_find(lfs, &cwd, &path, NULL);
///     if (tag < 0) {
///         return tag;
///     }
///
///     uint16_t id = lfs_tag_id(tag);
///     if (id == 0x3ff) {
///         // special case for root
///         id = 0;
///         int err = lfs_dir_fetch(lfs, &cwd, lfs->root);
///         if (err) {
///             return err;
///         }
///     }
///
///     tag = lfs_dir_get(lfs, &cwd, LFS_MKTAG(0x7ff, 0x3ff, 0),
///             LFS_MKTAG(LFS_TYPE_USERATTR + type,
///                 id, lfs_min(size, lfs->attr_max)),
///             buffer);
///     if (tag < 0) {
///         if (tag == LFS_ERR_NOENT) {
///             return LFS_ERR_NOATTR;
///         }
///
///         return tag;
///     }
///
///     return lfs_tag_size(tag);
/// }
/// ```
pub fn lfs_getattr_(
    lfs: *mut Lfs,
    path: &CStr,
    r#type: u8,
    buffer: *mut core::ffi::c_void,
    size: lfs_size_t,
) -> Result<lfs_size_t, Error> {
    if lfs.is_null() {
        return crate::lfs_err!(Err(Error::Invalid));
    }
    unsafe {
        let lfs = &mut *lfs;

        let mut cwd = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [lfs.root[0], lfs.root[1]],
        };

        let mut path_ptr = path;
        let tag = lfs_dir_find(lfs, &mut cwd, &mut path_ptr, &mut None)?;

        let mut id = lfs_tag_id(tag) as u16;
        if id == 0x3ff {
            id = 0;
            let lfs_root = borrow_unchecked(&lfs.root);
            lfs_dir_fetch(lfs, &mut cwd, lfs_root)?;
        }
        let size = lfs_min(size, (*lfs).attr_max);
        let gtag = lfs_mktag(LFS_TYPE_USERATTR + r#type as u32, id as u32, size);
        let buffer = core::slice::from_raw_parts_mut(buffer as *mut u8, size as usize);
        let tag = lfs_dir_get(lfs, &cwd, lfs_mktag(0x7ff, 0x3ff, 0), gtag, buffer);
        if let Err(err) = tag {
            if err == Error::NoEntry {
                return crate::lfs_err!(Err(Error::NoAttribute));
            }
            return tag;
        }

        Ok(lfs_tag_size(tag.unwrap()))
    }
}

/// Per lfs.c lfs_commitattr (lines 4141-4163)
///
/// Translation docs: Resolve path to (cwd, tag), get id (0 for root), commit one USERATTR.
///
/// C:
/// ```c
/// static int lfs_commitattr(lfs_t *lfs, const char *path,
///         uint8_t type, const void *buffer, lfs_size_t size) {
///     lfs_mdir_t cwd;
///     lfs_stag_t tag = lfs_dir_find(lfs, &cwd, &path, NULL);
///     if (tag < 0) {
///         return tag;
///     }
///
///     uint16_t id = lfs_tag_id(tag);
///     if (id == 0x3ff) {
///         // special case for root
///         id = 0;
///         int err = lfs_dir_fetch(lfs, &cwd, lfs->root);
///         if (err) {
///             return err;
///         }
///     }
///
///     return lfs_dir_commit(lfs, &cwd, LFS_MKATTRS(
///             {LFS_MKTAG(LFS_TYPE_USERATTR + type, id, size), buffer}));
/// }
/// #endif
/// ```
pub fn lfs_commitattr(
    lfs: &mut Lfs,
    path: &CStr,
    r#type: u8,
    buffer: &[u8],
    size: lfs_size_t,
) -> Result<(), Error> {
    unsafe {
        let lfs = &mut *lfs;

        let mut cwd = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [lfs.root[0], lfs.root[1]],
        };

        let mut path_ptr = path;
        let tag = lfs_dir_find(lfs, &mut cwd, &mut path_ptr, &mut None)?;

        let mut id = lfs_tag_id(tag) as u16;
        if id == 0x3ff {
            id = 0;
            let lfs_root = borrow_unchecked(&lfs.root);
            lfs_dir_fetch(lfs, &mut cwd, lfs_root)?;
        }

        let attrs = [lfs_mattr {
            tag: lfs_mktag(LFS_TYPE_USERATTR + r#type as u32, id as u32, size),
            buffer,
        }];
        lfs_dir_commit(lfs, &mut cwd, &attrs)
    }
}

/// Per lfs.c lfs_setattr_ (lines 4165-4174)
///
/// Translation docs: Check attr_max, then commit attr.
///
/// C:
/// ```c
/// static int lfs_setattr_(lfs_t *lfs, const char *path,
///         uint8_t type, const void *buffer, lfs_size_t size) {
///     if (size > lfs->attr_max) {
///         return LFS_ERR_NOSPC;
///     }
///
///     return lfs_commitattr(lfs, path, type, buffer, size);
/// }
/// #endif
/// ```
pub fn lfs_setattr_(
    lfs: &mut Lfs,
    path: &CStr,
    r#type: u8,
    buffer: &[u8],
    size: lfs_size_t,
) -> Result<(), Error> {
    if size > lfs.attr_max {
        return crate::lfs_err!(Err(Error::NoSpace));
    }
    lfs_commitattr(lfs, path, r#type, buffer, size)
}

/// Per lfs.c lfs_removeattr_ (lines 4176-4196)
///
/// Translation docs: Remove attr by committing with NULL buffer and size 0x3ff.
///
/// C:
/// ```c
/// static int lfs_removeattr_(lfs_t *lfs, const char *path, uint8_t type) {
///     return lfs_commitattr(lfs, path, type, NULL, 0x3ff);
/// }
/// #endif
/// ```
pub fn lfs_removeattr_(lfs: &mut Lfs, path: &CStr, r#type: u8) -> Result<(), Error> {
    lfs_commitattr(lfs, path, r#type, &[], 0x3ff)
}
