//! Directory find. Per lfs.c lfs_dir_find, lfs_dir_find_match.

use core::cmp;

use zerocopy::IntoBytes;

use crate::Storage;
use crate::bd::bd::lfs_bd_cmp;
use crate::borrow_unchecked::borrow_unchecked;
use crate::dir::LfsMdir;
use crate::dir::fetch::lfs_dir_fetchmatch;
use crate::dir::traverse::lfs_dir_get;
use crate::error::Error;
use crate::fs::Lfs;
use crate::lfs_type::lfs_type::{LFS_TYPE_GLOBALS, LFS_TYPE_NAME, LFS_TYPE_STRUCT, LFS_TYPE3_DIR};
use crate::tag::{lfs_diskoff, lfs_mktag, lfs_tag_id, lfs_tag_size, lfs_tag_type3};
use crate::types::lfs_tag_t;
use crate::util::{lfs_pair_fromle32, lfs_strcspn, lfs_strspn};

/// Per lfs.c struct lfs_dir_find_match (lines 1447-1475)
#[repr(C)]
pub struct LfsDirFindMatch<'a, S> {
    pub lfs: *mut Lfs<S>,
    pub name: &'a [u8],
}

/// Per lfs.c lfs_dir_find_match (and struct lfs_dir_find_match) (lines 1447-1475)
///
/// C:
/// ```c
/// struct lfs_dir_find_match {
///     lfs_t *lfs;
///     const void *name;
///     lfs_size_t size;
/// };
///
/// static int lfs_dir_find_match(void *data,
///         lfs_tag_t tag, const void *buffer) {
///     struct lfs_dir_find_match *name = data;
///     lfs_t *lfs = name->lfs;
///     const struct lfs_diskoff *disk = buffer;
///
///     // compare with disk
///     lfs_size_t diff = lfs_min(name->size, lfs_tag_size(tag));
///     int res = lfs_bd_cmp(lfs,
///             NULL, &lfs->rcache, diff,
///             disk->block, disk->off, name->name, diff);
///     if (res != LFS_CMP_EQ) {
///         return res;
///     }
///
///     // only equal if our size is still the same
///     if (name->size != lfs_tag_size(tag)) {
///         return (name->size < lfs_tag_size(tag)) ? LFS_CMP_LT : LFS_CMP_GT;
///     }
///
///     // found a match!
///     return LFS_CMP_EQ;
/// }
///
/// ```
pub async  fn lfs_dir_find_match<S: Storage>(
    name: &LfsDirFindMatch<'_, S>,
    tag: lfs_tag_t,
    disk: &lfs_diskoff,
) -> Result<cmp::Ordering, Error> {
    let lfs = unsafe { &mut *name.lfs };

    let diff = cmp::min(name.name.len(), lfs_tag_size(tag) as usize);
    let res = lfs_bd_cmp(
        lfs,
        None,
        unsafe { &mut *lfs.rcache.get() },
        diff,
        disk.block,
        disk.off as usize,
        &name.name[..diff],
    ).await;
    if res != Ok(core::cmp::Ordering::Equal) {
        return res;
    }

    Ok((name.name.len() as u32).cmp(&lfs_tag_size(tag)))
}

/// Per lfs.c lfs_dir_find (lines 1483-1590)
///
/// C:
/// ```c
/// static lfs_stag_t lfs_dir_find(lfs_t *lfs, lfs_mdir_t *dir,
///         const char **path, uint16_t *id) {
///     // we reduce path to a single name if we can find it
///     const char *name = *path;
///
///     // default to root dir
///     lfs_stag_t tag = LFS_MKTAG(LFS_TYPE_DIR, 0x3ff, 0);
///     dir->tail[0] = lfs->root[0];
///     dir->tail[1] = lfs->root[1];
///
///     // empty paths are not allowed
///     if (*name == '\0') {
///         return LFS_ERR_INVAL;
///     }
///
///     while (true) {
/// nextname:
///         // skip slashes if we're a directory
///         if (lfs_tag_type3(tag) == LFS_TYPE_DIR) {
///             name += strspn(name, "/");
///         }
///         lfs_size_t namelen = strcspn(name, "/");
///
///         // skip '.'
///         if (namelen == 1 && memcmp(name, ".", 1) == 0) {
///             name += namelen;
///             goto nextname;
///         }
///
///         // error on unmatched '..', trying to go above root?
///         if (namelen == 2 && memcmp(name, "..", 2) == 0) {
///             return LFS_ERR_INVAL;
///         }
///
///         // skip if matched by '..' in name
///         const char *suffix = name + namelen;
///         lfs_size_t sufflen;
///         int depth = 1;
///         while (true) {
///             suffix += strspn(suffix, "/");
///             sufflen = strcspn(suffix, "/");
///             if (sufflen == 0) {
///                 break;
///             }
///
///             if (sufflen == 1 && memcmp(suffix, ".", 1) == 0) {
///                 // noop
///             } else if (sufflen == 2 && memcmp(suffix, "..", 2) == 0) {
///                 depth -= 1;
///                 if (depth == 0) {
///                     name = suffix + sufflen;
///                     goto nextname;
///                 }
///             } else {
///                 depth += 1;
///             }
///
///             suffix += sufflen;
///         }
///
///         // found path
///         if (*name == '\0') {
///             return tag;
///         }
///
///         // update what we've found so far
///         *path = name;
///
///         // only continue if we're a directory
///         if (lfs_tag_type3(tag) != LFS_TYPE_DIR) {
///             return LFS_ERR_NOTDIR;
///         }
///
///         // grab the entry data
///         if (lfs_tag_id(tag) != 0x3ff) {
///             lfs_stag_t res = lfs_dir_get(lfs, dir, LFS_MKTAG(0x700, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_STRUCT, lfs_tag_id(tag), 8), dir->tail);
///             if (res < 0) {
///                 return res;
///             }
///             lfs_pair_fromle32(dir->tail);
///         }
///
///         // find entry matching name
///         while (true) {
///             tag = lfs_dir_fetchmatch(lfs, dir, dir->tail,
///                     LFS_MKTAG(0x780, 0, 0),
///                     LFS_MKTAG(LFS_TYPE_NAME, 0, namelen),
///                     id,
///                     lfs_dir_find_match, &(struct lfs_dir_find_match){
///                         lfs, name, namelen});
///             if (tag < 0) {
///                 return tag;
///             }
///
///             if (tag) {
///                 break;
///             }
///
///             if (!dir->split) {
///                 return LFS_ERR_NOENT;
///             }
///         }
///
///         // to next name
///         name += namelen;
///     }
/// }
/// ```
pub async  fn lfs_dir_find<S: Storage>(
    lfs: &mut Lfs<S>,
    dir: &mut LfsMdir,
    path: &mut &str,
    id: &mut Option<&mut u16>,
) -> Result<crate::types::lfs_tag_t, Error> {
    if path.is_empty() {
        return crate::lfs_err!(Err(Error::Invalid));
    }
    let mut name = path.as_bytes();

    // C: lfs.c:1488-1491
    let mut tag = lfs_mktag(LFS_TYPE3_DIR, 0x3ff, 0);
    dir.tail[0] = lfs.root[0];
    dir.tail[1] = lfs.root[1];

    // C: lfs.c:1494-1495
    if name[0] == 0 {
        return crate::lfs_err!(Err(Error::Invalid));
    }

    'nextname: loop {
        // C: nextname - lfs.c:1510-1512
        if (lfs_tag_type3(tag)) == LFS_TYPE3_DIR {
            let skip = lfs_strspn(name, b'/');
            name = &name[skip..];
        }
        let namelen = lfs_strcspn(name, b'/');

        // C: lfs.c:1516-1519 - skip '.'
        if namelen == 1 && name[0] == b'.' {
            name = &name[1..];
            continue;
        }

        // C: lfs.c:1522-1524 - error on '..' at top level
        if namelen == 2 && name[0] == b'.' && name[1] == b'.' {
            return crate::lfs_err!(Err(Error::Invalid));
        }

        // C: lfs.c:1527-1541 - skip if matched by '..' in path
        let mut suffix = &name[namelen..];
        let mut depth: i32 = 1;

        loop {
            let suffix_skip = lfs_strspn(suffix, b'/');
            suffix = &suffix[suffix_skip..];
            let sufflen = lfs_strcspn(suffix, b'/');
            if sufflen == 0 {
                break;
            }
            if sufflen == 1 && suffix[0] == b'.' {
                // noop
            } else if sufflen == 2 && suffix[0] == b'.' && suffix[1] == b'.' {
                depth -= 1;
                if depth == 0 {
                    name = &suffix[sufflen..];
                    continue 'nextname;
                }
            } else {
                depth += 1;
            }
            suffix = &suffix[sufflen..];
        }

        // C: lfs.c:1544-1546 - found path
        if name.is_empty() {
            return Ok(tag);
        }

        // C: lfs.c:1549
        *path = unsafe { str::from_utf8_unchecked(name) };

        // C: lfs.c:1652-1654
        if (lfs_tag_type3(tag)) != LFS_TYPE3_DIR {
            return crate::lfs_err!(Err(Error::NotDir));
        }

        // C: lfs.c:1557-1564
        if lfs_tag_id(tag as u32) != 0x3ff {
            let dir_tail = unsafe { borrow_unchecked(&mut dir.tail) };
            let res = lfs_dir_get(
                lfs,
                dir,
                lfs_mktag(LFS_TYPE_GLOBALS, 0x3ff, 0),
                lfs_mktag(LFS_TYPE_STRUCT, lfs_tag_id(tag as u32) as u32, 8),
                dir_tail.as_mut_bytes(),
            );
            res?;
            lfs_pair_fromle32(&mut dir.tail);
        }

        // C: lfs.c:1567-1584 - find entry matching name
        loop {
            let match_data = LfsDirFindMatch {
                lfs,
                name: &name[..namelen],
            };
            tag = lfs_dir_fetchmatch(
                lfs,
                dir,
                dir.tail,
                lfs_mktag(0x780, 0, 0),
                lfs_mktag(LFS_TYPE_NAME, 0, namelen),
                id,
                Some(&|tag, disk| lfs_dir_find_match(&match_data, tag, disk)),
            ).await?;

            if tag != 0 {
                break;
            }
            if !dir.split {
                return crate::lfs_err!(Err(Error::NoEntry));
            }
        }

        name = &name[namelen..];
    }
}
