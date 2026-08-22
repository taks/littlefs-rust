//! File operations. Per lfs.c lfs_file_opencfg_, lfs_file_close_, lfs_file_sync_, etc.

use core::ptr::NonNull;

use zerocopy::IntoBytes;

use crate::bd::bd::{lfs_bd_read, lfs_cache_drop, lfs_cache_zero};
use crate::dir::traverse::lfs_dir_getread;
use crate::error::Error;
use crate::file::LfsFile;
use crate::file::ctz::lfs_ctz_find;
use crate::lfs_info::LfsFileConfig;
use crate::lfs_type::OpenFlags;
use crate::lfs_type::lfs_type::{LFS_TYPE_INLINESTRUCT, LFS_TYPE3_REG};
use crate::tag::lfs_mktag;
use crate::types::LFS_BLOCK_INLINE;
use crate::types::{lfs_block_t, lfs_off_t};
use crate::util::lfs_min;
use crate::{Lfs, LfsAttr};

/// Per lfs.c lfs_file_opencfg_ (lines 3065-3236)
///
/// C:
/// ```c
/// static int lfs_file_opencfg_(lfs_t *lfs, lfs_file_t *file,
///         const char *path, int flags,
///         const struct lfs_file_config *cfg) {
/// #ifndef LFS_READONLY
///     // deorphan if we haven't yet, needed at most once after poweron
///     if ((flags & LFS_O_WRONLY) == LFS_O_WRONLY) {
///         int err = lfs_fs_forceconsistency(lfs);
///         if (err) {
///             return err;
///         }
///     }
/// #else
///     LFS_ASSERT((flags & LFS_O_RDONLY) == LFS_O_RDONLY);
/// #endif
///
///     // setup simple file details
///     int err;
///     file->cfg = cfg;
///     file->flags = flags;
///     file->pos = 0;
///     file->off = 0;
///     file->cache.buffer = NULL;
///
///     // allocate entry for file if it doesn't exist
///     lfs_stag_t tag = lfs_dir_find(lfs, &file->m, &path, &file->id);
///     if (tag < 0 && !(tag == LFS_ERR_NOENT && lfs_path_islast(path))) {
///         err = tag;
///         goto cleanup;
///     }
///
///     // get id, add to list of mdirs to catch update changes
///     file->type = LFS_TYPE_REG;
///     lfs_mlist_append(lfs, (struct lfs_mlist *)file);
///
/// #ifdef LFS_READONLY
///     if (tag == LFS_ERR_NOENT) {
///         err = LFS_ERR_NOENT;
///         goto cleanup;
/// #else
///     if (tag == LFS_ERR_NOENT) {
///         if (!(flags & LFS_O_CREAT)) {
///             err = LFS_ERR_NOENT;
///             goto cleanup;
///         }
///
///         // don't allow trailing slashes
///         if (lfs_path_isdir(path)) {
///             err = LFS_ERR_NOTDIR;
///             goto cleanup;
///         }
///
///         // check that name fits
///         lfs_size_t nlen = lfs_path_namelen(path);
///         if (nlen > lfs->name_max) {
///             err = LFS_ERR_NAMETOOLONG;
///             goto cleanup;
///         }
///
///         // get next slot and create entry to remember name
///         err = lfs_dir_commit(lfs, &file->m, LFS_MKATTRS(
///                 {LFS_MKTAG(LFS_TYPE_CREATE, file->id, 0), NULL},
///                 {LFS_MKTAG(LFS_TYPE_REG, file->id, nlen), path},
///                 {LFS_MKTAG(LFS_TYPE_INLINESTRUCT, file->id, 0), NULL}));
///
///         // it may happen that the file name doesn't fit in the metadata blocks, e.g., a 256 byte file name will
///         // not fit in a 128 byte block.
///         err = (err == LFS_ERR_NOSPC) ? LFS_ERR_NAMETOOLONG : err;
///         if (err) {
///             goto cleanup;
///         }
///
///         tag = LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, 0);
///     } else if (flags & LFS_O_EXCL) {
///         err = LFS_ERR_EXIST;
///         goto cleanup;
/// #endif
///     } else if (lfs_tag_type3(tag) != LFS_TYPE_REG) {
///         err = LFS_ERR_ISDIR;
///         goto cleanup;
/// #ifndef LFS_READONLY
///     } else if (flags & LFS_O_TRUNC) {
///         // truncate if requested
///         tag = LFS_MKTAG(LFS_TYPE_INLINESTRUCT, file->id, 0);
///         file->flags |= LFS_F_DIRTY;
/// #endif
///     } else {
///         // try to load what's on disk, if it's inlined we'll fix it later
///         tag = lfs_dir_get(lfs, &file->m, LFS_MKTAG(0x700, 0x3ff, 0),
///                 LFS_MKTAG(LFS_TYPE_STRUCT, file->id, 8), &file->ctz);
///         if (tag < 0) {
///             err = tag;
///             goto cleanup;
///         }
///         lfs_ctz_fromle32(&file->ctz);
///     }
///
///     // fetch attrs
///     for (unsigned i = 0; i < file->cfg->attr_count; i++) {
///         // if opened for read / read-write operations
///         if ((file->flags & LFS_O_RDONLY) == LFS_O_RDONLY) {
///             lfs_stag_t res = lfs_dir_get(lfs, &file->m,
///                     LFS_MKTAG(0x7ff, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_USERATTR + file->cfg->attrs[i].type,
///                         file->id, file->cfg->attrs[i].size),
///                         file->cfg->attrs[i].buffer);
///             if (res < 0 && res != LFS_ERR_NOENT) {
///                 err = res;
///                 goto cleanup;
///             }
///         }
///
/// #ifndef LFS_READONLY
///         // if opened for write / read-write operations
///         if ((file->flags & LFS_O_WRONLY) == LFS_O_WRONLY) {
///             if (file->cfg->attrs[i].size > lfs->attr_max) {
///                 err = LFS_ERR_NOSPC;
///                 goto cleanup;
///             }
///
///             file->flags |= LFS_F_DIRTY;
///         }
/// #endif
///     }
///
///     // allocate buffer if needed
///     if (file->cfg->buffer) {
///         file->cache.buffer = file->cfg->buffer;
///     } else {
///         file->cache.buffer = lfs_malloc(lfs->cfg->cache_size);
///         if (!file->cache.buffer) {
///             err = LFS_ERR_NOMEM;
///             goto cleanup;
///         }
///     }
///
///     // zero to avoid information leak
///     lfs_cache_zero(lfs, &file->cache);
///
///     if (lfs_tag_type3(tag) == LFS_TYPE_INLINESTRUCT) {
///         // load inline files
///         file->ctz.head = LFS_BLOCK_INLINE;
///         file->ctz.size = lfs_tag_size(tag);
///         file->flags |= LFS_F_INLINE;
///         file->cache.block = file->ctz.head;
///         file->cache.off = 0;
///         file->cache.size = lfs->cfg->cache_size;
///
///         // don't always read (may be new/trunc file)
///         if (file->ctz.size > 0) {
///             lfs_stag_t res = lfs_dir_get(lfs, &file->m,
///                     LFS_MKTAG(0x700, 0x3ff, 0),
///                     LFS_MKTAG(LFS_TYPE_STRUCT, file->id,
///                         lfs_min(file->cache.size, 0x3fe)),
///                     file->cache.buffer);
///             if (res < 0) {
///                 err = res;
///                 goto cleanup;
///             }
///         }
///     }
///
///     return 0;
///
/// cleanup:
///     // clean up lingering resources
/// #ifndef LFS_READONLY
///     file->flags |= LFS_F_ERRED;
/// #endif
///     lfs_file_close_(lfs, file);
///     return err;
/// }
/// ```
pub fn lfs_file_opencfg_(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    path: &str,
    flags: OpenFlags,
    cfg: &mut LfsFileConfig,
) -> Result<(), Error> {
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::dir::find::lfs_dir_find;
    use crate::dir::lfs_mlist::lfs_mlist_append;
    use crate::dir::traverse::lfs_dir_get;
    use crate::file::lfs_ctz::lfs_ctz_fromle32;
    use crate::fs::superblock::lfs_fs_forceconsistency;
    use crate::lfs_type::lfs_type::{LFS_TYPE_CREATE, LFS_TYPE_REG, LFS_TYPE_USERATTR};
    use crate::tag::{lfs_mktag, lfs_tag_size, lfs_tag_type3};
    use crate::types::LFS_BLOCK_INLINE;
    use crate::util::{lfs_min, lfs_path_isdir, lfs_path_islast, lfs_path_namelen};

    if flags.contains(OpenFlags::WRITE) {
        lfs_fs_forceconsistency(lfs)?;
    }

    file.cfg = unsafe { core::mem::transmute(&mut *cfg) };
    file.flags = flags;
    file.pos = 0;
    file.off = 0;
    file.cache.buffer = NonNull::from_ref(&[]);

    let mut path_ptr = path;
    let mut tag = lfs_dir_find(lfs, &mut file.m, &mut path_ptr, &mut Some(&mut file.id));
    if let Err(err) = tag
        && !(err == Error::NoEntry && lfs_path_islast(path_ptr.as_bytes()))
    {
        let _ = lfs_file_close_(lfs, file);
        return crate::lfs_pass_err!(Err(err));
    }

    file.type_ = LFS_TYPE_REG;
    lfs_mlist_append(lfs, unsafe { file.as_mut_lsf_mist() });

    if tag == Err(Error::NoEntry) {
        if !flags.contains(OpenFlags::CREATE) {
            let _ = lfs_file_close_(lfs, file);
            return crate::lfs_err!(Err(Error::NoEntry));
        }
        if lfs_path_isdir(path_ptr.as_bytes()) {
            let _ = lfs_file_close_(lfs, file);
            return Err(Error::NotDir);
        }
        let nlen = lfs_path_namelen(path_ptr.as_bytes());
        if nlen > lfs.name_max {
            let _ = lfs_file_close_(lfs, file);
            return crate::lfs_err!(Err(Error::NameTooLong));
        }
        lfs_alloc_ckpoint(lfs);
        let attrs = [
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, file.id as u32, 0),
                buffer: &[],
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE3_REG, file.id as u32, nlen),
                buffer: path_ptr.as_bytes(),
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_INLINESTRUCT, file.id as u32, 0),
                buffer: &[],
            },
        ];
        let err = crate::dir::commit::lfs_dir_commit(lfs, &mut file.m, &attrs);
        let err = if err == Err(Error::NoSpace) {
            Err(Error::NameTooLong)
        } else {
            err
        };
        if err.is_err() {
            let _ = lfs_file_close_(lfs, file);
            return crate::lfs_pass_err!(err);
        }
    } else if flags.contains(OpenFlags::EXCL) {
        let _ = lfs_file_close_(lfs, file);
        return crate::lfs_err!(Err(Error::Exists));
    } else if (lfs_tag_type3(tag.unwrap())) != LFS_TYPE3_REG {
        let _ = lfs_file_close_(lfs, file);
        return crate::lfs_err!(Err(Error::IsDir));
    } else if flags.contains(OpenFlags::TRUNC) {
        // C: lfs.c:100-104 — truncate if requested
        tag = Ok(lfs_mktag(LFS_TYPE_INLINESTRUCT, file.id as u32, 0));
        file.flags.insert(OpenFlags::DIRTY);
    } else {
        // C: tag = lfs_dir_get(...) — overwrite tag with STRUCT tag for later use
        let struct_tag = lfs_dir_get(
            lfs,
            &file.m,
            lfs_mktag(0x700, 0x3ff, 0),
            lfs_mktag(
                crate::lfs_type::lfs_type::LFS_TYPE_STRUCT,
                file.id as u32,
                8,
            ),
            file.ctz.as_mut_bytes(),
        );
        if let Err(err) = struct_tag {
            let _ = lfs_file_close_(lfs, file);
            return Err(err);
        }
        tag = struct_tag;
        lfs_ctz_fromle32(&mut file.ctz);
    }

    // C: lfs.c:3162-3187 — fetch attrs
    let attrs = &mut cfg.attrs;
    for attr in attrs.iter_mut() {
        if file.flags.contains(OpenFlags::READ) {
            let res = lfs_dir_get(
                lfs,
                &file.m,
                lfs_mktag(0x7ff, 0x3ff, 0),
                lfs_mktag(
                    LFS_TYPE_USERATTR + attr.type_ as u16,
                    file.id as u32,
                    attr.buffer.len() as u32,
                ),
                attr.buffer,
            );
            if let Err(err) = res
                && err != Error::NoEntry
            {
                let _ = lfs_file_close_(lfs, file);
                return Err(err);
            }
        }
        if file.flags.contains(OpenFlags::WRITE) {
            if attr.buffer.len() as u32 > lfs.attr_max {
                let _ = lfs_file_close_(lfs, file);
                return crate::lfs_err!(Err(Error::NoSpace));
            }
            file.flags.insert(OpenFlags::DIRTY);
        }
    }

    if !(cfg).buffer.is_empty() {
        file.cache.buffer = NonNull::from_mut(cfg.buffer);
    } else {
        #[cfg(feature = "alloc")]
        {
            unsafe {
                let size = lfs.cfg.as_ref().expect("cfg").cache_size;
                let data = crate::lfs_alloc_module::lfs_malloc(size);
                file.cache.buffer =
                    NonNull::from_mut(core::slice::from_raw_parts_mut(data, size as usize));
            }
        }
        #[cfg(not(feature = "alloc"))]
        {
            lfs_file_close_(lfs, file);
            return crate::lfs_err!(Err(Error::NoMemory));
        }

        if file.cache.buffer.is_empty() {
            let _ = lfs_file_close_(lfs, file);
            return crate::lfs_err!(Err(Error::NoMemory));
        }
    }

    lfs_cache_zero(lfs, &mut file.cache);

    let tag_val = if tag == Err(Error::NoEntry) {
        lfs_mktag(LFS_TYPE_INLINESTRUCT, 0, 0)
    } else {
        tag.unwrap()
    };
    if (lfs_tag_type3(tag_val as u32)) == LFS_TYPE_INLINESTRUCT {
        file.ctz.head = LFS_BLOCK_INLINE;
        file.ctz.size = if tag == Err(Error::NoEntry) {
            0
        } else {
            lfs_tag_size(tag_val as u32)
        };
        file.flags.insert(OpenFlags::INLINE);
        file.cache.block = file.ctz.head;
        file.cache.off = 0;
        file.cache.size = unsafe { lfs.cfg.as_ref().expect("cfg").cache_size };
        if file.ctz.size > 0 {
            let res = lfs_dir_get(
                lfs,
                &file.m,
                lfs_mktag(0x700, 0x3ff, 0),
                lfs_mktag(
                    crate::lfs_type::lfs_type::LFS_TYPE_STRUCT,
                    file.id as u32,
                    lfs_min(file.cache.size, 0x3fe),
                ),
                unsafe { &mut file.cache.buffer.as_mut()[..file.cache.size as usize] },
            );
            if let Err(err) = res {
                let _ = lfs_file_close_(lfs, file);
                return Err(err);
            }
        }
    }

    Ok(())
}

/// Per lfs.c lfs_file_open_ (lines 3238-3244)
///
/// C: Wrapper that calls opencfg with default config.
/// Static defaults for lfs_file_open (no opencfg). C uses the same;
/// a stack-local would make file.cfg a dangling pointer after return.
static mut BUFFER: [u8; 0] = [];
static mut ATTRS: [LfsAttr; 0] = [];
#[allow(clippy::deref_addrof)]
static mut LFS_FILE_DEFAULTS: LfsFileConfig = LfsFileConfig {
    buffer: unsafe { &mut *(&raw mut BUFFER) },
    attrs: unsafe { &mut *(&raw mut ATTRS) },
    // attr_count: 0,
};

#[allow(clippy::deref_addrof)]
pub fn lfs_file_open_(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    path: &str,
    flags: OpenFlags,
) -> Result<(), Error> {
    lfs_file_opencfg_(lfs, file, path, flags, unsafe {
        &mut *(&raw mut LFS_FILE_DEFAULTS)
    })
}

/// Per lfs.c lfs_file_close_ (lines 3246-3264)
///
/// Translation docs: Sync if dirty, remove from mlist, free cache buffer if we allocated it.
///
/// C: lfs.c:3246-3264
pub fn lfs_file_close_(lfs: &mut crate::fs::Lfs, file: &mut LfsFile) -> Result<(), Error> {
    use crate::dir::lfs_mlist::lfs_mlist_remove;

    let mut err = Ok(());
    if !file.flags.contains(OpenFlags::ERRED) {
        err = lfs_file_sync_(lfs, file);
    }

    unsafe { lfs_mlist_remove(lfs, file.as_mut_lsf_mist()) };

    #[cfg(feature = "alloc")]
    unsafe {
        let cfg = file.cfg.as_ref().unwrap();
        if (cfg.buffer).is_empty() && !file.cache.buffer.is_empty() {
            crate::lfs_alloc_module::lfs_free(
                file.cache.buffer.as_mut().as_mut_ptr(),
                lfs.cfg.as_ref().expect("cfg").cache_size,
            );
        }
    }

    err
}

/// Translation docs: Relocates file data into a new block. For inline reads via
/// lfs_dir_getread; for CTZ via lfs_bd_read. Writes with lfs_bd_prog. Retries on LFS_ERR_CORRUPT.
///
/// Per lfs.c lfs_file_relocate (lines 3266-3335)
///
/// C:
/// ```c
/// static int lfs_file_relocate(lfs_t *lfs, lfs_file_t *file) {
///     while (true) {
///         // just relocate what exists into new block
///         lfs_block_t nblock;
///         int err = lfs_alloc(lfs, &nblock);
///         if (err) {
///             return err;
///         }
///
///         err = lfs_bd_erase(lfs, nblock);
///         if (err) {
///             if (err == LFS_ERR_CORRUPT) {
///                 goto relocate;
///             }
///             return err;
///         }
///
///         // either read from dirty cache or disk
///         for (lfs_off_t i = 0; i < file->off; i++) {
///             uint8_t data;
///             if (file->flags & LFS_F_INLINE) {
///                 err = lfs_dir_getread(lfs, &file->m,
///                         // note we evict inline files before they can be dirty
///                         NULL, &file->cache, file->off-i,
///                         LFS_MKTAG(0xfff, 0x1ff, 0),
///                         LFS_MKTAG(LFS_TYPE_INLINESTRUCT, file->id, 0),
///                         i, &data, 1);
///                 if (err) {
///                     return err;
///                 }
///             } else {
///                 err = lfs_bd_read(lfs,
///                         &file->cache, &lfs->rcache, file->off-i,
///                         file->block, i, &data, 1);
///                 if (err) {
///                     return err;
///                 }
///             }
///
///             err = lfs_bd_prog(lfs,
///                     &lfs->pcache, &lfs->rcache, true,
///                     nblock, i, &data, 1);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 return err;
///             }
///         }
///
///         // copy over new state of file
///         memcpy(file->cache.buffer, lfs->pcache.buffer, lfs->cfg->cache_size);
///         file->cache.block = lfs->pcache.block;
///         file->cache.off = lfs->pcache.off;
///         file->cache.size = lfs->pcache.size;
///         lfs_cache_zero(lfs, &lfs->pcache);
///
///         file->block = nblock;
///         file->flags |= LFS_F_WRITING;
///         return 0;
///
/// relocate:
///         LFS_DEBUG("Bad block at 0x%"PRIx32, nblock);
///
///         // just clear cache and try a new block
///         lfs_cache_drop(lfs, &lfs->pcache);
///     }
/// }
/// ```
pub fn lfs_file_relocate(lfs: &mut crate::fs::Lfs, file: &mut LfsFile) -> Result<(), Error> {
    use crate::bd::bd::{lfs_bd_erase, lfs_bd_prog, lfs_bd_read, lfs_cache_drop, lfs_cache_zero};
    use crate::block_alloc::alloc::{lfs_alloc, lfs_alloc_lookahead};

    'relocate: loop {
        let mut nblock: lfs_block_t = 0;
        lfs_alloc(lfs, &mut nblock)?;

        let err = lfs_bd_erase(lfs, nblock);
        if let Err(e) = err {
            if e == Error::Corrupt {
                let _ = lfs_alloc_lookahead(lfs, nblock);
                lfs_cache_drop(lfs, unsafe { &mut *lfs.pcache.get() });
                continue 'relocate;
            }
            return crate::lfs_pass_err!(err);
        }

        for i in 0..file.off {
            let mut data = [0u8];
            let err = if file.flags.contains(OpenFlags::INLINE) {
                let gtag = lfs_mktag(LFS_TYPE_INLINESTRUCT, file.id as u32, 0);
                lfs_dir_getread(
                    lfs,
                    &file.m,
                    core::ptr::null(),
                    &mut file.cache,
                    file.off - i,
                    lfs_mktag(0xfff, 0x1ff, 0),
                    gtag,
                    i,
                    &mut data,
                )
            } else {
                lfs_bd_read(
                    lfs,
                    Some(&file.cache),
                    unsafe { &mut *lfs.rcache.get() },
                    file.off - i,
                    file.block,
                    i,
                    &mut data,
                )
            };
            crate::lfs_pass_err!(err)?;

            let err = lfs_bd_prog(
                lfs,
                unsafe { &mut *lfs.pcache.get() },
                unsafe { &mut *lfs.rcache.get() },
                true,
                nblock,
                i,
                &data,
            );
            if let Err(err) = err {
                if err == Error::Corrupt {
                    let _ = lfs_alloc_lookahead(lfs, nblock);
                    lfs_cache_drop(lfs, unsafe { &mut *lfs.pcache.get() });
                    continue 'relocate;
                }
                return crate::lfs_pass_err!(Err(err));
            }
        }

        {
            let pcache = lfs.pcache.get_mut();

            unsafe {
                let cache_size = lfs.cfg.as_ref().expect("cfg").cache_size as usize;
                file.cache.buffer.as_mut()[..cache_size]
                    .copy_from_slice(&pcache.buffer.as_ref()[..cache_size]);
            }
            file.cache.block = pcache.block;
            file.cache.off = pcache.off;
            file.cache.size = pcache.size;
            file.block = nblock;
            file.flags.insert(OpenFlags::WRITING);
        }
        lfs_cache_zero(lfs, unsafe { &mut *lfs.pcache.get() });
        return Ok(());
    }
}

/// Translation docs: Converts an inline file to CTZ when it exceeds inline_max.
/// Sets off=pos, alloc ckpoint, relocate, clears LFS_F_INLINE.
///
/// Per lfs.c lfs_file_outline (lines 3337-3348)
///
/// C:
/// ```c
/// static int lfs_file_outline(lfs_t *lfs, lfs_file_t *file) {
///     file->off = file->pos;
///     unsafe { lfs_alloc_ckpoint(lfs) };
///     int err = lfs_file_relocate(lfs, file);
///     if (err) {
///         return err;
///     }
///
///     file->flags &= ~LFS_F_INLINE;
///     return 0;
/// }
/// ```
pub fn lfs_file_outline(lfs: &mut crate::fs::Lfs, file: &mut LfsFile) -> Result<(), Error> {
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;

    file.off = file.pos;

    lfs_alloc_ckpoint(lfs);
    lfs_file_relocate(lfs, file)?;

    file.flags.remove(OpenFlags::INLINE);

    Ok(())
}

/// Per lfs.c lfs_file_flush (lines 3350-3429)
///
/// C:
/// ```c
/// static int lfs_file_flush(lfs_t *lfs, lfs_file_t *file) {
///     if (file->flags & LFS_F_READING) {
///         if (!(file->flags & LFS_F_INLINE)) {
///             lfs_cache_drop(lfs, &file->cache);
///         }
///         file->flags &= ~LFS_F_READING;
///     }
///
/// #ifndef LFS_READONLY
///     if (file->flags & LFS_F_WRITING) {
///         lfs_off_t pos = file->pos;
///
///         if (!(file->flags & LFS_F_INLINE)) {
///             // copy over anything after current branch
///             lfs_file_t orig = {
///                 .ctz.head = file->ctz.head,
///                 .ctz.size = file->ctz.size,
///                 .flags = LFS_O_RDONLY,
///                 .pos = file->pos,
///                 .cache = lfs->rcache,
///             };
///             lfs_cache_drop(lfs, &lfs->rcache);
///
///             while (file->pos < file->ctz.size) {
///                 // copy over a byte at a time, leave it up to caching
///                 // to make this efficient
///                 uint8_t data;
///                 lfs_ssize_t res = lfs_file_flushedread(lfs, &orig, &data, 1);
///                 if (res < 0) {
///                     return res;
///                 }
///
///                 res = lfs_file_flushedwrite(lfs, file, &data, 1);
///                 if (res < 0) {
///                     return res;
///                 }
///
///                 // keep our reference to the rcache in sync
///                 if (lfs->rcache.block != LFS_BLOCK_NULL) {
///                     lfs_cache_drop(lfs, &orig.cache);
///                     lfs_cache_drop(lfs, &lfs->rcache);
///                 }
///             }
///
///             // write out what we have
///             while (true) {
///                 int err = lfs_bd_flush(lfs, &file->cache, &lfs->rcache, true);
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         goto relocate;
///                     }
///                     return err;
///                 }
///
///                 break;
///
/// relocate:
///                 LFS_DEBUG("Bad block at 0x%"PRIx32, file->block);
///                 err = lfs_file_relocate(lfs, file);
///                 if (err) {
///                     return err;
///                 }
///             }
///         } else {
///             file->pos = lfs_max(file->pos, file->ctz.size);
///         }
///
///         // actual file updates
///         file->ctz.head = file->block;
///         file->ctz.size = file->pos;
///         file->flags &= ~LFS_F_WRITING;
///         file->flags |= LFS_F_DIRTY;
///
///         file->pos = pos;
///     }
/// #endif
///
///     return 0;
/// }
/// ```
pub fn lfs_file_flush(lfs: &mut crate::fs::Lfs, file: &mut LfsFile) -> Result<(), Error> {
    use crate::bd::bd::lfs_bd_flush;
    use crate::util::lfs_max;

    unsafe {
        if file.flags.contains(OpenFlags::READING) {
            if !file.flags.contains(OpenFlags::INLINE) {
                lfs_cache_drop(lfs, &mut file.cache);
            }
            file.flags.remove(OpenFlags::READING);
        }

        if file.flags.contains(OpenFlags::WRITING) {
            let pos = file.pos;
            if file.flags.contains(OpenFlags::INLINE) {
                file.pos = lfs_max(pos, file.ctz.size);
            } else {
                let mut orig = LfsFile {
                    next: core::ptr::null_mut(),
                    id: file.id,
                    type_: file.type_,
                    m: core::mem::zeroed(),
                    ctz: crate::file::lfs_ctz::LfsCtz {
                        head: file.ctz.head,
                        size: file.ctz.size,
                    },
                    flags: OpenFlags::READ,
                    pos: file.pos,
                    block: 0,
                    off: 0,
                    cache: core::ptr::read(lfs.rcache.get()),
                    cfg: core::ptr::null(),
                };
                lfs_cache_drop(lfs, &mut *lfs.rcache.get());

                #[allow(clippy::while_immutable_condition)] // file.pos updated by flushedwrite
                while file.pos < file.ctz.size {
                    let mut data: u8 = 0;
                    let _res = lfs_file_flushedread(lfs, &mut orig, data.as_mut_bytes())?;

                    let _res = lfs_file_flushedwrite(lfs, file, data.as_bytes())?;

                    if lfs.rcache.get_mut().block != crate::types::LFS_BLOCK_NULL {
                        lfs_cache_drop(lfs, &mut orig.cache);
                        lfs_cache_drop(lfs, &mut *lfs.rcache.get());
                    }
                }

                'flush: loop {
                    let err = lfs_bd_flush(lfs, &mut file.cache, &mut *lfs.rcache.get(), true);
                    if let Err(err) = err {
                        if err == Error::Corrupt {
                            lfs_file_relocate(lfs, file)?;

                            continue 'flush;
                        }
                        return crate::lfs_pass_err!(Err(err));
                    }
                    break;
                }
            }
            file.ctz.head = file.block;
            file.ctz.size = file.pos;
            file.flags.remove(OpenFlags::WRITING);
            file.flags.insert(OpenFlags::DIRTY);
            file.pos = pos;
        }
    }
    Ok(())
}

/// Per lfs.c lfs_file_sync_ (lines 3431-3490)
///
/// C:
/// ```c
/// static int lfs_file_sync_(lfs_t *lfs, lfs_file_t *file) {
///     if (file->flags & LFS_F_ERRED) {
///         // it's not safe to do anything if our file errored
///         return 0;
///     }
///
///     int err = lfs_file_flush(lfs, file);
///     if (err) {
///         file->flags |= LFS_F_ERRED;
///         return err;
///     }
///
///     if ((file->flags & LFS_F_DIRTY) &&
///             !lfs_pair_isnull(file->m.pair)) {
///         // before we commit metadata, we need sync the disk to make sure
///         // data writes don't complete after metadata writes
///         if (!(file->flags & LFS_F_INLINE)) {
///             err = lfs_bd_sync(lfs, &lfs->pcache, &lfs->rcache, false);
///             if (err) {
///                 return err;
///             }
///         }
///
///         // update dir entry
///         uint16_t type;
///         const void *buffer;
///         lfs_size_t size;
///         struct lfs_ctz ctz;
///         if (file->flags & LFS_F_INLINE) {
///             // inline the whole file
///             type = LFS_TYPE_INLINESTRUCT;
///             buffer = file->cache.buffer;
///             size = file->ctz.size;
///         } else {
///             // update the ctz reference
///             type = LFS_TYPE_CTZSTRUCT;
///             // copy ctz so alloc will work during a relocate
///             ctz = file->ctz;
///             lfs_ctz_tole32(&ctz);
///             buffer = &ctz;
///             size = sizeof(ctz);
///         }
///
///         // commit file data and attributes
///         err = lfs_dir_commit(lfs, &file->m, LFS_MKATTRS(
///                 {LFS_MKTAG(type, file->id, size), buffer},
///                 {LFS_MKTAG(LFS_FROM_USERATTRS, file->id,
///                     file->cfg->attr_count), file->cfg->attrs}));
///         if (err) {
///             file->flags |= LFS_F_ERRED;
///             return err;
///         }
///
///         file->flags &= ~LFS_F_DIRTY;
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_file_sync_(lfs: &mut crate::fs::Lfs, file: &mut LfsFile) -> Result<(), Error> {
    use crate::dir::commit::lfs_dir_commit;
    use crate::tag::lfs_mktag;
    use crate::util::lfs_pair_isnull;

    unsafe {
        if file.flags.contains(OpenFlags::ERRED) {
            return Ok(());
        }

        let err = lfs_file_flush(lfs, file);
        if err.is_err() {
            file.flags.insert(OpenFlags::ERRED);
            return crate::lfs_pass_err!(err);
        }

        if file.flags.contains(OpenFlags::DIRTY) && !lfs_pair_isnull(&file.m.pair) {
            if !file.flags.contains(OpenFlags::INLINE) {
                crate::bd::bd::lfs_bd_sync(
                    lfs,
                    &mut *lfs.pcache.get(),
                    &mut *lfs.rcache.get(),
                    false,
                )?;
            }

            // C: copy ctz so alloc will work during a relocate
            // Must live through lfs_dir_commit — declared outside the if/else
            let mut ctz = file.ctz;
            let (type_, buffer, size) = if file.flags.contains(OpenFlags::INLINE) {
                (
                    LFS_TYPE_INLINESTRUCT,
                    &file.cache.buffer.as_ref()[..file.ctz.size as usize],
                    file.ctz.size,
                )
            } else {
                crate::file::lfs_ctz::lfs_ctz_tole32(&mut ctz);
                (
                    crate::lfs_type::lfs_type::LFS_TYPE_CTZSTRUCT,
                    ctz.as_bytes(),
                    core::mem::size_of::<crate::file::LfsCtz>() as u32,
                )
            };

            let attrs = [
                crate::tag::lfs_mattr {
                    tag: lfs_mktag(type_, file.id as u32, size),
                    buffer,
                },
                crate::tag::lfs_mattr {
                    tag: lfs_mktag(
                        crate::lfs_type::lfs_type::LFS_FROM_USERATTRS,
                        file.id as u32,
                        file.cfg.as_ref().map_or(0, |c| c.attrs.len() as u32),
                    ) as u32,
                    buffer: file.cfg.as_ref().map_or(&[], |c| {
                        if c.attrs.is_empty() {
                            &[]
                        } else {
                            c.attrs.as_bytes()
                        }
                    }),
                },
            ];
            let err = lfs_dir_commit(lfs, &mut file.m, &attrs);
            if err.is_err() {
                file.flags.insert(OpenFlags::ERRED);
                return crate::lfs_pass_err!(err);
            }
            file.flags.remove(OpenFlags::DIRTY);
        }
    }
    Ok(())
}

/// Per lfs.c lfs_file_flushedread (lines 3492-3551)
///
/// Translation docs: Read file data. Handles inline and CTZ files.
/// Uses file cache for block caching; dir_getread for inline, bd_read for CTZ.
///
/// C: lfs.c:3493-3551
pub fn lfs_file_flushedread(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    buffer: &mut [u8],
) -> Result<crate::types::lfs_size_t, Error> {
    if buffer.is_empty() {
        return Ok(0);
    }

    unsafe {
        let cfg = lfs.cfg.as_ref().expect("cfg");
        let block_size = cfg.block_size;

        if file.pos >= file.ctz.size {
            return Ok(0);
        }

        let size = lfs_min(buffer.len() as u32, file.ctz.size - file.pos);
        let mut nsize = size;

        let mut data = buffer;
        while nsize > 0 {
            if !file.flags.contains(OpenFlags::READING) || file.off == block_size {
                if !file.flags.contains(OpenFlags::INLINE) {
                    lfs_ctz_find(
                        lfs,
                        None,
                        &mut file.cache,
                        file.ctz.head,
                        file.ctz.size,
                        file.pos,
                        &mut file.block,
                        &mut file.off,
                    )?;
                } else {
                    file.block = LFS_BLOCK_INLINE;
                    file.off = file.pos;
                }
                file.flags.insert(OpenFlags::READING);
            }

            let diff = lfs_min(nsize, block_size - file.off);
            if file.flags.contains(OpenFlags::INLINE) {
                let gtag = lfs_mktag(LFS_TYPE_INLINESTRUCT, file.id as u32, 0);
                lfs_dir_getread(
                    lfs,
                    &file.m,
                    core::ptr::null(),
                    &mut file.cache,
                    block_size,
                    lfs_mktag(0xfff, 0x1ff, 0),
                    gtag,
                    file.off,
                    &mut data[..diff as usize],
                )?;
            } else {
                lfs_bd_read(
                    lfs,
                    None,
                    &mut file.cache,
                    block_size,
                    file.block,
                    file.off,
                    &mut data[..(diff as usize)],
                )?;
            }

            file.pos += diff;
            file.off += diff;
            data = &mut data[(diff as usize)..];
            nsize -= diff;
        }

        Ok(size as crate::types::lfs_size_t)
    }
}

/// Per lfs.c lfs_file_read_ (lines 3553-3570)
///
/// Translation docs: Read file. Asserts RDONLY; flushes pending writes if any; delegates to flushedread.
///
/// C: lfs.c:3553-3570
pub fn lfs_file_read_(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    buffer: &mut [u8],
) -> Result<crate::types::lfs_size_t, Error> {
    crate::lfs_assert!(file.flags.contains(OpenFlags::READ));

    if file.flags.contains(OpenFlags::WRITING) {
        lfs_file_flush(lfs, file)?;
    }

    lfs_file_flushedread(lfs, file, buffer)
}

/// Translation docs: Writes file data. Outlines inline files that exceed inline_max.
/// For CTZ: ctz_find when extending, ctz_extend for new blocks, relocate on CORRUPT.
///
/// Per lfs.c lfs_file_flushedwrite (lines 3572-3654)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_file_flushedwrite(lfs_t *lfs, lfs_file_t *file,
///         const void *buffer, lfs_size_t size) {
///     const uint8_t *data = buffer;
///     lfs_size_t nsize = size;
///
///     if ((file->flags & LFS_F_INLINE) &&
///             lfs_max(file->pos+nsize, file->ctz.size) > lfs->inline_max) {
///         // inline file doesn't fit anymore
///         int err = lfs_file_outline(lfs, file);
///         if (err) {
///             file->flags |= LFS_F_ERRED;
///             return err;
///         }
///     }
///
///     while (nsize > 0) {
///         // check if we need a new block
///         if (!(file->flags & LFS_F_WRITING) ||
///                 file->off == lfs->cfg->block_size) {
///             if (!(file->flags & LFS_F_INLINE)) {
///                 if (!(file->flags & LFS_F_WRITING) && file->pos > 0) {
///                     // find out which block we're extending from
///                     int err = lfs_ctz_find(lfs, NULL, &file->cache,
///                             file->ctz.head, file->ctz.size,
///                             file->pos-1, &file->block, &(lfs_off_t){0});
///                     if (err) {
///                         file->flags |= LFS_F_ERRED;
///                         return err;
///                     }
///
///                     // mark cache as dirty since we may have read data into it
///                     lfs_cache_zero(lfs, &file->cache);
///                 }
///
///                 // extend file with new blocks
///                 unsafe { lfs_alloc_ckpoint(lfs) };
///                 int err = lfs_ctz_extend(lfs, &file->cache, &lfs->rcache,
///                         file->block, file->pos,
///                         &file->block, &file->off);
///                 if (err) {
///                     file->flags |= LFS_F_ERRED;
///                     return err;
///                 }
///             } else {
///                 file->block = LFS_BLOCK_INLINE;
///                 file->off = file->pos;
///             }
///
///             file->flags |= LFS_F_WRITING;
///         }
///
///         // program as much as we can in current block
///         lfs_size_t diff = lfs_min(nsize, lfs->cfg->block_size - file->off);
///         while (true) {
///             int err = lfs_bd_prog(lfs, &file->cache, &lfs->rcache, true,
///                     file->block, file->off, data, diff);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 file->flags |= LFS_F_ERRED;
///                 return err;
///             }
///
///             break;
/// relocate:
///             err = lfs_file_relocate(lfs, file);
///             if (err) {
///                 file->flags |= LFS_F_ERRED;
///                 return err;
///             }
///         }
///
///         file->pos += diff;
///         file->off += diff;
///         data += diff;
///         nsize -= diff;
///
///         unsafe { lfs_alloc_ckpoint(lfs) };
///     }
///
///     return size;
/// }
/// ```
pub fn lfs_file_flushedwrite(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    buffer: &[u8],
) -> Result<crate::types::lfs_size_t, Error> {
    use crate::bd::bd::{lfs_bd_prog, lfs_cache_zero};
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::file::ctz::{lfs_ctz_extend, lfs_ctz_find};

    if buffer.is_empty() {
        return Ok(0);
    }

    unsafe {
        let cfg = lfs.cfg.as_ref().expect("cfg");
        let block_size = cfg.block_size;
        let mut nsize = buffer.len() as u32;

        if file.flags.contains(OpenFlags::INLINE)
            && crate::util::lfs_max(file.pos + nsize, file.ctz.size) > lfs.inline_max
        {
            let err = lfs_file_outline(lfs, file);
            if let Err(err) = err {
                file.flags.insert(OpenFlags::ERRED);
                return Err(err);
            }
        }

        let mut data = buffer;
        while nsize > 0 {
            if !file.flags.contains(OpenFlags::WRITING) || file.off == block_size {
                if file.flags.contains(OpenFlags::INLINE) {
                    file.block = LFS_BLOCK_INLINE;
                    file.off = file.pos;
                } else {
                    if !file.flags.contains(OpenFlags::WRITING) && file.pos > 0 {
                        let mut block_off: lfs_off_t = 0;
                        let err = lfs_ctz_find(
                            lfs,
                            None,
                            &mut *lfs.rcache.get(),
                            file.ctz.head,
                            file.ctz.size,
                            file.pos - 1,
                            &mut file.block,
                            &mut block_off,
                        );
                        if let Err(err) = err {
                            file.flags.insert(OpenFlags::ERRED);
                            return Err(err);
                        }
                        lfs_cache_zero(lfs, &mut file.cache);
                    }
                    lfs_alloc_ckpoint(lfs);
                    let err = lfs_ctz_extend(
                        lfs,
                        &mut file.cache,
                        &mut *lfs.rcache.get(),
                        file.block,
                        file.pos,
                        &mut file.block,
                        &mut file.off,
                    );
                    if let Err(err) = err {
                        file.flags.insert(OpenFlags::ERRED);
                        return Err(err);
                    }
                }
                file.flags.insert(OpenFlags::WRITING);
            }

            let diff = lfs_min(nsize, block_size - file.off);
            'prog: loop {
                let err = lfs_bd_prog(
                    lfs,
                    &mut file.cache,
                    &mut *lfs.rcache.get(),
                    true,
                    file.block,
                    file.off,
                    &data[..(diff as usize)],
                );
                if let Err(err) = err {
                    if err == Error::Corrupt {
                        let err = lfs_file_relocate(lfs, file);
                        if let Err(err) = err {
                            file.flags.insert(OpenFlags::ERRED);
                            return Err(err);
                        }
                        continue 'prog;
                    }
                    file.flags.insert(OpenFlags::ERRED);
                    return Err(err);
                }
                break;
            }

            file.pos += diff;
            file.off += diff;
            data = &data[(diff as usize)..];
            nsize -= diff;

            lfs_alloc_ckpoint(lfs);
        }
        Ok(buffer.len() as u32)
    }
}

/// Per lfs.c lfs_file_write_ (lines 3656-3698)
///
/// C:
/// ```c
/// static lfs_ssize_t lfs_file_write_(lfs_t *lfs, lfs_file_t *file,
///         const void *buffer, lfs_size_t size) {
///     LFS_ASSERT((file->flags & LFS_O_WRONLY) == LFS_O_WRONLY);
///
///     if (file->flags & LFS_F_READING) {
///         // drop any reads
///         int err = lfs_file_flush(lfs, file);
///         if (err) {
///             return err;
///         }
///     }
///
///     if ((file->flags & LFS_O_APPEND) && file->pos < file->ctz.size) {
///         file->pos = file->ctz.size;
///     }
///
///     if (file->pos + size > lfs->file_max) {
///         // Larger than file limit?
///         return LFS_ERR_FBIG;
///     }
///
///     if (!(file->flags & LFS_F_WRITING) && file->pos > file->ctz.size) {
///         // fill with zeros
///         lfs_off_t pos = file->pos;
///         file->pos = file->ctz.size;
///
///         while (file->pos < pos) {
///             lfs_ssize_t res = lfs_file_flushedwrite(lfs, file, &(uint8_t){0}, 1);
///             if (res < 0) {
///                 return res;
///             }
///         }
///     }
///
///     lfs_ssize_t nsize = lfs_file_flushedwrite(lfs, file, buffer, size);
///     if (nsize < 0) {
///         return nsize;
///     }
///
///     file->flags &= ~LFS_F_ERRED;
///     return nsize;
/// }
/// ```
pub fn lfs_file_write_(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    buffer: &[u8],
) -> Result<crate::types::lfs_size_t, Error> {
    crate::lfs_assert!(file.flags.contains(OpenFlags::WRITE));

    if file.flags.contains(OpenFlags::READING) {
        lfs_file_flush(lfs, file)?;
    }
    if file.flags.contains(OpenFlags::APPEND) && file.pos < file.ctz.size {
        file.pos = file.ctz.size;
    }
    if file.pos + (buffer.len() as u32) > lfs.file_max {
        return Err(Error::FileTooBig);
    }

    // C: lfs.c:3677-3688 — zero-fill gap when writing past end of file
    if !file.flags.contains(OpenFlags::WRITING) && file.pos > file.ctz.size {
        let pos = file.pos;
        file.pos = file.ctz.size;
        let zero: u8 = 0;
        #[allow(clippy::while_immutable_condition)] // pos mutated via raw ptr in flushedwrite
        while file.pos < pos {
            let _res = lfs_file_flushedwrite(lfs, file, zero.as_bytes())?;
        }
    }

    let nsize = lfs_file_flushedwrite(lfs, file, buffer)?;

    file.flags.remove(OpenFlags::ERRED);
    Ok(nsize)
}

/// Per lfs.c lfs_file_seek_ (lines 3700-3751)
///
/// Translation docs: Seek to new position. SEEK_SET, SEEK_CUR, SEEK_END.
/// May avoid flush if new pos is in current cache (reading path).
///
/// C: lfs.c:3700-3751
pub fn lfs_file_seek_(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    off: crate::types::lfs_soff_t,
    whence: i32,
) -> Result<crate::types::lfs_off_t, Error> {
    use crate::file::ctz::lfs_ctz_index;
    use crate::lfs_type::lfs_whence_flags::{LFS_SEEK_CUR, LFS_SEEK_END, LFS_SEEK_SET};

    unsafe {
        let file_max = lfs.file_max;
        let block_size = lfs.cfg.as_ref().expect("cfg").block_size;

        let mut npos = file.pos;
        if whence == LFS_SEEK_SET {
            npos = off as lfs_off_t;
        } else if whence == LFS_SEEK_CUR {
            npos = (file.pos as i64 + off as i64) as lfs_off_t;
        } else if whence == LFS_SEEK_END {
            npos = (lfs_file_size_(lfs, file) as i64 + off as i64) as lfs_off_t;
        }

        if npos > file_max {
            return crate::lfs_err!(Err(Error::Invalid));
        }

        if file.pos == npos {
            return Ok(npos as _);
        }

        if file.flags.contains(OpenFlags::READING) && file.off != block_size {
            let mut opos = file.pos;
            let mut npos_off = npos;
            let oindex = lfs_ctz_index(lfs, &mut opos);
            let nindex = lfs_ctz_index(lfs, &mut npos_off);
            if oindex == nindex
                && npos_off >= file.cache.off
                && npos_off < file.cache.off + file.cache.size
            {
                file.pos = npos;
                file.off = npos_off;
                return Ok(npos as _);
            }
        }

        lfs_file_flush(lfs, file)?;

        file.pos = npos;
        Ok(npos as crate::types::lfs_off_t)
    }
}

/// Translation docs: Truncates file to size. Shrink: revert to inline if size <= inline_max,
/// else flush and update CTZ head/size. Grow: seek end, write zeros. Restores pos.
///
/// Per lfs.c lfs_file_truncate_ (lines 3753-3838)
///
/// C:
/// ```c
/// static int lfs_file_truncate_(lfs_t *lfs, lfs_file_t *file, lfs_off_t size) {
///     LFS_ASSERT((file->flags & LFS_O_WRONLY) == LFS_O_WRONLY);
///
///     if (size > LFS_FILE_MAX) {
///         return LFS_ERR_INVAL;
///     }
///
///     lfs_off_t pos = file->pos;
///     lfs_off_t oldsize = lfs_file_size_(lfs, file);
///     if (size < oldsize) {
///         // revert to inline file?
///         if (size <= lfs->inline_max) {
///             // flush+seek to head
///             lfs_soff_t res = lfs_file_seek_(lfs, file, 0, LFS_SEEK_SET);
///             if (res < 0) {
///                 return (int)res;
///             }
///
///             // read our data into rcache temporarily
///             lfs_cache_drop(lfs, &lfs->rcache);
///             res = lfs_file_flushedread(lfs, file,
///                     lfs->rcache.buffer, size);
///             if (res < 0) {
///                 return (int)res;
///             }
///
///             file->ctz.head = LFS_BLOCK_INLINE;
///             file->ctz.size = size;
///             file->flags |= LFS_F_DIRTY | LFS_F_READING | LFS_F_INLINE;
///             file->cache.block = file->ctz.head;
///             file->cache.off = 0;
///             file->cache.size = lfs->cfg->cache_size;
///             memcpy(file->cache.buffer, lfs->rcache.buffer, size);
///
///         } else {
///             // need to flush since directly changing metadata
///             int err = lfs_file_flush(lfs, file);
///             if (err) {
///                 return err;
///             }
///
///             // lookup new head in ctz skip list
///             err = lfs_ctz_find(lfs, NULL, &file->cache,
///                     file->ctz.head, file->ctz.size,
///                     size-1, &file->block, &(lfs_off_t){0});
///             if (err) {
///                 return err;
///             }
///
///             // need to set pos/block/off consistently so seeking back to
///             // the old position does not get confused
///             file->pos = size;
///             file->ctz.head = file->block;
///             file->ctz.size = size;
///             file->flags |= LFS_F_DIRTY | LFS_F_READING;
///         }
///     } else if (size > oldsize) {
///         // flush+seek if not already at end
///         lfs_soff_t res = lfs_file_seek_(lfs, file, 0, LFS_SEEK_END);
///         if (res < 0) {
///             return (int)res;
///         }
///
///         // fill with zeros
///         while (file->pos < size) {
///             res = lfs_file_write_(lfs, file, &(uint8_t){0}, 1);
///             if (res < 0) {
///                 return (int)res;
///             }
///         }
///     }
///
///     // restore pos
///     lfs_soff_t res = lfs_file_seek_(lfs, file, pos, LFS_SEEK_SET);
///     if (res < 0) {
///       return (int)res;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_file_truncate_(
    lfs: &mut crate::fs::Lfs,
    file: &mut LfsFile,
    size: lfs_off_t,
) -> Result<(), Error> {
    use crate::file::ctz::lfs_ctz_find;
    use crate::lfs_type::lfs_whence_flags::{LFS_SEEK_END, LFS_SEEK_SET};

    crate::lfs_assert!(file.flags.contains(OpenFlags::WRITE));

    if size > lfs.file_max {
        return Err(Error::Invalid);
    }

    let pos = file.pos;
    let oldsize = lfs_file_size_(lfs, file) as lfs_off_t;

    if size < oldsize {
        if size <= lfs.inline_max {
            // C: lfs.c:3762-3786 — revert to inline
            let _res = lfs_file_seek_(lfs, file, 0, LFS_SEEK_SET)?;

            // Read existing data from CTZ blocks into rcache temporarily
            crate::bd::bd::lfs_cache_drop(lfs, unsafe { &mut *lfs.rcache.get() });
            let buffer = unsafe { &mut lfs.rcache.get_mut().buffer.as_mut()[..size as usize] };
            let _res = lfs_file_flushedread(lfs, file, buffer)?;

            file.ctz.head = LFS_BLOCK_INLINE;
            file.ctz.size = size;
            file.flags
                .insert(OpenFlags::DIRTY | OpenFlags::READING | OpenFlags::INLINE);
            file.cache.block = file.ctz.head;
            file.cache.off = 0;
            file.cache.size = unsafe { lfs.cfg.as_ref().expect("cfg").cache_size };

            // Copy data from rcache into file cache
            unsafe {
                file.cache.buffer.as_mut()[..size as usize]
                    .copy_from_slice(&lfs.rcache.get_mut().buffer.as_ref()[..size as usize]);
            }
        } else {
            // C: lfs.c:3787-3806 — shrink CTZ
            lfs_file_flush(lfs, file)?;

            let mut off_zero: lfs_off_t = 0;
            lfs_ctz_find(
                lfs,
                None,
                &mut file.cache,
                file.ctz.head,
                file.ctz.size,
                size.saturating_sub(1),
                &mut file.block,
                &mut off_zero,
            )?;

            file.pos = size;
            file.ctz.head = file.block;
            file.ctz.size = size;
            file.flags.insert(OpenFlags::DIRTY | OpenFlags::READING);
        }
    } else if size > oldsize {
        // C: lfs.c:3807-3818 — grow
        let _res = lfs_file_seek_(lfs, file, 0, LFS_SEEK_END)?;

        let zero = [0u8];
        #[allow(clippy::while_immutable_condition)] // file.pos updated by lfs_file_write_
        while file.pos < size {
            let _res = lfs_file_write_(lfs, file, &zero)?;
        }
    }

    let _res = lfs_file_seek_(lfs, file, pos as i32, LFS_SEEK_SET)?;

    Ok(())
}

/// Per lfs.c lfs_file_tell_ (lines 3835-3838)
///
/// Translation docs: Returns the current file position.
///
/// C:
/// ```c
/// static lfs_soff_t lfs_file_tell_(lfs_t *lfs, lfs_file_t *file) {
///     (void)lfs;
///     return file->pos;
/// }
/// ```
pub fn lfs_file_tell_(_lfs: *const core::ffi::c_void, file: &LfsFile) -> crate::types::lfs_soff_t {
    file.pos as crate::types::lfs_soff_t
}

/// Per lfs.c lfs_file_rewind_ (lines 3840-3850)
///
/// Translation docs: Seek to start of file.
///
/// C: lfs.c:3840-3850
pub fn lfs_file_rewind_(lfs: &mut crate::fs::Lfs, file: &mut LfsFile) -> Result<(), Error> {
    lfs_file_seek_(
        lfs,
        file,
        0,
        crate::lfs_type::lfs_whence_flags::LFS_SEEK_SET,
    )?;
    Ok(())
}

/// Per lfs.c lfs_file_size_ (lines 3849-3858)
///
/// C:
/// ```c
/// static lfs_soff_t lfs_file_size_(lfs_t *lfs, lfs_file_t *file) {
///     (void)lfs;
/// #ifndef LFS_READONLY
///     if (file->flags & LFS_F_WRITING) {
///         return lfs_max(file->pos, file->ctz.size);
///     }
/// #endif
///     return file->ctz.size;
/// }
/// ```
pub fn lfs_file_size_(_lfs: &Lfs, file: &LfsFile) -> crate::types::lfs_soff_t {
    if file.flags.contains(OpenFlags::WRITING) {
        return crate::util::lfs_max(file.pos, file.ctz.size) as crate::types::lfs_soff_t;
    }
    file.ctz.size as crate::types::lfs_soff_t
}
