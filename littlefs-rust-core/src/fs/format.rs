//! Format. Per lfs.c lfs_format_.

use core::cmp;

use zerocopy::IntoBytes;

use crate::block_alloc::alloc::lfs_alloc_ckpoint;
use crate::dir::LfsMdir;
use crate::dir::commit::lfs_dir_alloc;
use crate::dir::commit::lfs_dir_commit;
use crate::dir::fetch::lfs_dir_fetch;
use crate::error::Error;
use crate::fs::init::{lfs_deinit, lfs_init};
use crate::lfs_superblock::LfsSuperblock;
use crate::lfs_superblock::lfs_superblock_tole32;
use crate::lfs_type::lfs_type::{LFS_TYPE_CREATE, LFS_TYPE_INLINESTRUCT, LFS_TYPE_SUPERBLOCK};
use crate::tag::lfs_mktag;
use crate::types::LFS_DISK_VERSION;

/// Per lfs.c lfs_format_ (lines 4391-4462)
///
/// C:
/// ```c
/// static int lfs_format_(lfs_t *lfs, const struct lfs_config *cfg) {
///     int err = 0;
///     {
///         err = lfs_init(lfs, cfg);
///         if (err) {
///             return err;
///         }
///
///         LFS_ASSERT(cfg->block_count != 0);
///
///         // create free lookahead
///         memset(lfs->lookahead.buffer, 0, lfs->cfg->lookahead_size);
///         lfs->lookahead.start = 0;
///         lfs->lookahead.size = lfs_min(8*lfs->cfg->lookahead_size,
///                 lfs->block_count);
///         lfs->lookahead.next = 0;
///         unsafe { lfs_alloc_ckpoint(lfs as *mut _) };
///
///         // create root dir
///         lfs_mdir_t root;
///         err = lfs_dir_alloc(lfs, &root);
///         if (err) {
///             goto cleanup;
///         }
///
///         // write one superblock
///         lfs_superblock_t superblock = {
///             .version     = lfs_fs_disk_version(lfs),
///             .block_size  = lfs->cfg->block_size,
///             .block_count = lfs->block_count,
///             .name_max    = lfs->name_max,
///             .file_max    = lfs->file_max,
///             .attr_max    = lfs->attr_max,
///         };
///
///         lfs_superblock_tole32(&superblock);
///         err = lfs_dir_commit(lfs, &root, LFS_MKATTRS(
///                 {LFS_MKTAG(LFS_TYPE_CREATE, 0, 0), NULL},
///                 {LFS_MKTAG(LFS_TYPE_SUPERBLOCK, 0, 8), "littlefs"},
///                 {LFS_MKTAG(LFS_TYPE_INLINESTRUCT, 0, sizeof(superblock)),
///                     &superblock}));
///         if (err) {
///             goto cleanup;
///         }
///
///         // force compaction to prevent accidentally mounting any
///         // older version of littlefs that may live on disk
///         root.erased = false;
///         err = lfs_dir_commit(lfs, &root, NULL, 0);
///         if (err) {
///             goto cleanup;
///         }
///
///         // sanity check that fetch works
///         err = lfs_dir_fetch(lfs, &root, (const lfs_block_t[2]){0, 1});
///         if (err) {
///             goto cleanup;
///         }
///     }
///
/// cleanup:
///     lfs_deinit(lfs);
///     return err;
///
/// }
/// #endif
///
/// struct lfs_tortoise_t {
///     lfs_block_t pair[2];
///     lfs_size_t i;
///     lfs_size_t period;
/// };
/// ```
pub fn lfs_format_(
    lfs: &mut super::lfs::Lfs,
    cfg: &crate::lfs_config::LfsConfig,
) -> Result<(), Error> {
    let mut err = lfs_init(lfs, cfg);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    unsafe {
        crate::lfs_assert!(cfg.block_count != 0);

        // create free lookahead
        lfs.lookahead.buffer.as_mut().fill(0);
        lfs.lookahead.start = 0;
        lfs.lookahead.size = cmp::min(
            8 * cfg.lookahead_buffer.unwrap().len() as u32,
            lfs.block_count,
        );
        lfs.lookahead.next = 0;
        lfs_alloc_ckpoint(lfs);

        // create root dir
        let mut root = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 0,
            etag: 0,
            count: 0,
            erased: false,
            split: false,
            tail: [0, 0],
        };
        err = lfs_dir_alloc(lfs, &mut root);
        if err.is_err() {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(err);
        }

        // write one superblock
        let mut superblock = LfsSuperblock {
            version: LFS_DISK_VERSION,
            block_size: cfg.block_size,
            block_count: lfs.block_count,
            name_max: lfs.name_max,
            file_max: lfs.file_max,
            attr_max: lfs.attr_max,
        };
        lfs_superblock_tole32(&mut superblock);

        let magic = b"littlefs";
        let attrs = [
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, 0, 0),
                buffer: &[],
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_SUPERBLOCK, 0, 8),
                buffer: magic,
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(
                    LFS_TYPE_INLINESTRUCT,
                    0,
                    core::mem::size_of::<LfsSuperblock>(),
                ),
                buffer: superblock.as_bytes(),
            },
        ];
        err = lfs_dir_commit(lfs, &mut root, &attrs);
        if err.is_err() {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(err);
        }

        // force compaction to prevent accidentally mounting any
        // older version of littlefs that may live on disk
        root.erased = false;
        err = lfs_dir_commit(lfs, &mut root, &[]);
        if err.is_err() {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(err);
        }

        // sanity check that fetch works
        err = lfs_dir_fetch(lfs, &mut root, [0, 1]);
        if err.is_err() {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(err);
        }
    }

    let _ = lfs_deinit(lfs);
    Ok(())
}

/// Test helper: init, alloc root, run traverse with format attrs, collect callback data.
/// Returns 0 on success; fills *out with tag_type1 and first_byte per callback.
///
/// # Safety
/// Caller must ensure `lfs` points to valid (e.g. zeroed) `Lfs`, `cfg` to valid `LfsConfig`,
/// and `out` to valid `TraverseTestOut` for the duration of the call.
pub unsafe fn test_traverse_format_attrs(
    lfs: &mut super::lfs::Lfs,
    cfg: &crate::lfs_config::LfsConfig,
    out: *mut crate::dir::traverse::TraverseTestOut,
) -> Result<(), Error> {
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::dir::commit::lfs_dir_alloc;
    use crate::dir::traverse::{lfs_dir_traverse, lfs_dir_traverse_test_cb};
    use crate::fs::init::{lfs_deinit, lfs_init};
    use crate::lfs_type::lfs_type::{LFS_TYPE_CREATE, LFS_TYPE_INLINESTRUCT, LFS_TYPE_SUPERBLOCK};
    use crate::tag::lfs_mktag;

    let mut err = lfs_init(lfs, cfg);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    unsafe {
        lfs.lookahead.buffer.as_mut().fill(0);
        lfs.lookahead.start = 0;
        lfs.lookahead.size = cmp::min(
            8 * cfg.lookahead_buffer.unwrap().len() as u32,
            lfs.block_count,
        );
        lfs.lookahead.next = 0;
        lfs_alloc_ckpoint(lfs);

        let mut root = LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 4,
            etag: 0xffff_ffff,
            count: 0,
            erased: false,
            split: false,
            tail: [0, 0],
        };
        err = lfs_dir_alloc(lfs, &mut root);
        if err.is_err() {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(err);
        }

        let magic = b"littlefs";
        let mut superblock = crate::lfs_superblock::LfsSuperblock {
            version: crate::types::LFS_DISK_VERSION,
            block_size: cfg.block_size,
            block_count: lfs.block_count,
            name_max: lfs.name_max,
            file_max: lfs.file_max,
            attr_max: lfs.attr_max,
        };
        crate::lfs_superblock::lfs_superblock_tole32(&mut superblock);

        let attrs = [
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, 0, 0),
                buffer: &[],
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_SUPERBLOCK, 0, 8),
                buffer: magic,
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(
                    LFS_TYPE_INLINESTRUCT,
                    0,
                    core::mem::size_of::<crate::lfs_superblock::LfsSuperblock>(),
                ),
                buffer: superblock.as_bytes(),
            },
        ];

        let err = lfs_dir_traverse(
            lfs,
            &root,
            0,
            0xffff_ffff,
            &attrs,
            0,
            0,
            0,
            0,
            0,
            lfs_dir_traverse_test_cb,
            out as *mut core::ffi::c_void,
        );
        if let Err(err) = err {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(Err(err));
        }
    }

    let _ = lfs_deinit(lfs);
    Ok(())
}

/// Test helper: same as test_traverse_format_attrs but with tmask that triggers push
/// (compact-style). Verifies that after push, the callback still receives SUPERBLOCK
/// with correct buffer (first byte 'l').
///
/// # Safety
/// Same as `test_traverse_format_attrs`.
pub unsafe fn test_traverse_filter_gets_superblock_after_push(
    lfs: &mut super::lfs::Lfs,
    cfg: &crate::lfs_config::LfsConfig,
    out: *mut crate::dir::traverse::TraverseTestOut,
) -> Result<(), Error> {
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::dir::commit::lfs_dir_alloc;
    use crate::dir::traverse::{lfs_dir_traverse, lfs_dir_traverse_test_cb};
    use crate::fs::init::{lfs_deinit, lfs_init};
    use crate::lfs_type::lfs_type::{
        LFS_TYPE_CREATE, LFS_TYPE_INLINESTRUCT, LFS_TYPE_NAME, LFS_TYPE_SUPERBLOCK,
    };
    use crate::tag::lfs_mktag;

    let mut err = lfs_init(lfs, cfg);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    unsafe {
        lfs.lookahead.buffer.as_mut().fill(0);
        lfs.lookahead.start = 0;
        lfs.lookahead.size = cmp::min(
            8 * cfg.lookahead_buffer.unwrap().len() as u32,
            lfs.block_count,
        );
        lfs.lookahead.next = 0;
        lfs_alloc_ckpoint(lfs);

        let mut root = crate::dir::LfsMdir {
            pair: [0, 0],
            rev: 0,
            off: 4,
            etag: 0xffff_ffff,
            count: 0,
            erased: false,
            split: false,
            tail: [0, 0],
        };
        err = lfs_dir_alloc(lfs, &mut root);
        if err.is_err() {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(err);
        }

        let magic = b"littlefs";
        let mut superblock = crate::lfs_superblock::LfsSuperblock {
            version: crate::types::LFS_DISK_VERSION,
            block_size: cfg.block_size,
            block_count: lfs.block_count,
            name_max: lfs.name_max,
            file_max: lfs.file_max,
            attr_max: lfs.attr_max,
        };
        crate::lfs_superblock::lfs_superblock_tole32(&mut superblock);

        let attrs = [
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, 0, 0),
                buffer: &[],
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(LFS_TYPE_SUPERBLOCK, 0, 8),
                buffer: magic,
            },
            crate::tag::lfs_mattr {
                tag: lfs_mktag(
                    LFS_TYPE_INLINESTRUCT,
                    0,
                    core::mem::size_of::<crate::lfs_superblock::LfsSuperblock>(),
                ),
                buffer: superblock.as_bytes(),
            },
        ];

        let err = lfs_dir_traverse(
            lfs,
            &root,
            0,
            0xffff_ffff,
            &attrs,
            lfs_mktag(0x400, 0x3ff, 0),
            lfs_mktag(LFS_TYPE_NAME, 0, 0),
            0,
            1,
            0,
            lfs_dir_traverse_test_cb,
            out as *mut core::ffi::c_void,
        );
        if let Err(err) = err {
            let _ = lfs_deinit(lfs);
            return crate::lfs_pass_err!(Err(err));
        }
    }

    let _ = lfs_deinit(lfs);
    Ok(())
}

/// Bypass test: write CREATE+SUPERBLOCK directly via commitattr, skip traverse.
/// If this produces correct magic at offset 12, the bug is in lfs_dir_traverse.
///
/// # Safety
/// Caller must ensure `lfs` points to valid (e.g. zeroed) `Lfs` and `cfg` to valid
/// `LfsConfig` for the duration of the call.
pub unsafe fn test_format_minimal_superblock(
    lfs: &mut super::lfs::Lfs,
    cfg: &crate::lfs_config::LfsConfig,
) -> Result<(), Error> {
    use crate::bd::bd::{lfs_bd_erase, lfs_bd_sync};
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::dir::LfsCommit;
    use crate::dir::commit::{
        lfs_dir_alloc, lfs_dir_commitattr, lfs_dir_commitcrc, lfs_dir_commitprog,
    };
    use crate::fs::init::{lfs_deinit, lfs_init};
    use crate::lfs_type::lfs_type::{LFS_TYPE_CREATE, LFS_TYPE_SUPERBLOCK};
    use crate::tag::lfs_mktag;

    let mut err = lfs_init(lfs, cfg);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    crate::lfs_assert!(cfg.block_count != 0);

    unsafe { lfs.lookahead.buffer.as_mut().fill(0) };
    lfs.lookahead.start = 0;
    lfs.lookahead.size = cmp::min(
        8 * cfg.lookahead_buffer.unwrap().len() as u32,
        lfs.block_count,
    );
    lfs.lookahead.next = 0;
    lfs_alloc_ckpoint(lfs);

    let mut root = LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: 0,
        etag: 0,
        count: 0,
        erased: false,
        split: false,
        tail: [0, 0],
    };
    err = lfs_dir_alloc(lfs, &mut root);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    // Write to block 1 (compact-style), skip traverse. pair is [1,0] or [0,1]
    // depending on alloc order; use pair[1] which receives the first compact write.
    let block = root.pair[1];
    let err = lfs_bd_erase(lfs, block);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    let end = cfg.block_size - 8;
    let mut commit = LfsCommit {
        block,
        off: 0,
        ptag: 0xffff_ffff,
        crc: 0xffff_ffff,
        begin: 0,
        end,
    };

    let rev = 1u32;
    let rev_le = rev.to_le();
    let err = lfs_dir_commitprog(lfs, &mut commit, rev_le.as_bytes());
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }
    commit.ptag = rev & 0x7fff_ffff;

    let magic = b"littlefs";
    let err = lfs_dir_commitattr(lfs, &mut commit, lfs_mktag(LFS_TYPE_CREATE, 0, 0), &[]);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }
    let err = lfs_dir_commitattr(
        lfs,
        &mut commit,
        lfs_mktag(LFS_TYPE_SUPERBLOCK, 0, 8),
        magic,
    );
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    let err = lfs_dir_commitcrc(lfs, &mut commit);
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    let err = lfs_bd_sync(
        lfs,
        unsafe { &mut *lfs.pcache.get() },
        unsafe { &mut *lfs.rcache.get() },
        false,
    );
    if err.is_err() {
        let _ = lfs_deinit(lfs);
        return crate::lfs_pass_err!(err);
    }

    let _ = lfs_deinit(lfs);
    Ok(())
}
