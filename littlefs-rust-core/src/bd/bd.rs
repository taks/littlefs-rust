//! Block device operations. Per lfs.c lfs_bd_read, lfs_bd_prog, lfs_bd_crc, etc.

use core::cmp;

use crate::bd::LfsCache;
use crate::error::Error;
use crate::fs::Lfs;
use crate::lfs_pass_err;
use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t};
use crate::util::{lfs_aligndown, lfs_alignup};

/// Per lfs.c lfs_cache_drop (lines 31-36)
///
/// C:
/// ```c
/// static inline void lfs_cache_drop(lfs_t *lfs, lfs_cache_t *rcache) {
///     // do not zero, cheaper if cache is readonly or only going to be
///     // written with identical data (during relocates)
///     (void)lfs;
///     rcache->block = LFS_BLOCK_NULL;
/// }
/// ```
#[inline(always)]
pub fn lfs_cache_drop(_lfs: &Lfs, rcache: &mut LfsCache) {
    rcache.block = crate::types::LFS_BLOCK_NULL;
}

/// Per lfs.c lfs_cache_zero (lines 38-42)
///
/// C:
/// ```c
/// static inline void lfs_cache_zero(lfs_t *lfs, lfs_cache_t *pcache) {
///     // zero to avoid information leak
///     memset(pcache->buffer, 0xff, lfs->cfg->cache_size);
///     pcache->block = LFS_BLOCK_NULL;
/// }
/// ```
#[inline(always)]
pub fn lfs_cache_zero(_lfs: &Lfs, pcache: &mut LfsCache) {
    unsafe {
        pcache.buffer.as_mut().fill(0xff);
    }
    pcache.block = crate::types::LFS_BLOCK_NULL;
}

/// Per lfs.c lfs_bd_read (lines 44-126)
///
/// C:
/// ```c
/// static int lfs_bd_read(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache, lfs_size_t hint,
///         lfs_block_t block, lfs_off_t off,
///         void *buffer, lfs_size_t size) {
///     uint8_t *data = buffer;
///     if (off+size > lfs->cfg->block_size
///             || (lfs->block_count && block >= lfs->block_count)) {
///         return LFS_ERR_CORRUPT;
///     }
///
///     while (size > 0) {
///         lfs_size_t diff = size;
///
///         if (pcache && block == pcache->block &&
///                 off < pcache->off + pcache->size) {
///             if (off >= pcache->off) {
///                 // is already in pcache?
///                 diff = lfs_min(diff, pcache->size - (off-pcache->off));
///                 memcpy(data, &pcache->buffer[off-pcache->off], diff);
///
///                 data += diff;
///                 off += diff;
///                 size -= diff;
///                 continue;
///             }
///
///             // pcache takes priority
///             diff = lfs_min(diff, pcache->off-off);
///         }
///
///         if (block == rcache->block &&
///                 off < rcache->off + rcache->size) {
///             if (off >= rcache->off) {
///                 // is already in rcache?
///                 diff = lfs_min(diff, rcache->size - (off-rcache->off));
///                 memcpy(data, &rcache->buffer[off-rcache->off], diff);
///
///                 data += diff;
///                 off += diff;
///                 size -= diff;
///                 continue;
///             }
///
///             // rcache takes priority
///             diff = lfs_min(diff, rcache->off-off);
///         }
///
///         if (size >= hint && off % lfs->cfg->read_size == 0 &&
///                 size >= lfs->cfg->read_size) {
///             // bypass cache?
///             diff = lfs_aligndown(diff, lfs->cfg->read_size);
///             int err = lfs->cfg->read(lfs->cfg, block, off, data, diff);
///             LFS_ASSERT(err <= 0);
///             if (err) {
///                 return err;
///             }
///
///             data += diff;
///             off += diff;
///             size -= diff;
///             continue;
///         }
///
///         // load to cache, first condition can no longer fail
///         LFS_ASSERT(!lfs->block_count || block < lfs->block_count);
///         rcache->block = block;
///         rcache->off = lfs_aligndown(off, lfs->cfg->read_size);
///         rcache->size = lfs_min(
///                 lfs_min(
///                     lfs_alignup(off+hint, lfs->cfg->read_size),
///                     lfs->cfg->block_size)
///                 - rcache->off,
///                 lfs->cfg->cache_size);
///         int err = lfs->cfg->read(lfs->cfg, rcache->block,
///                 rcache->off, rcache->buffer, rcache->size);
///         LFS_ASSERT(err <= 0);
///         if (err) {
///             return err;
///         }
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_bd_read(
    lfs: &Lfs,
    pcache: Option<&LfsCache>,
    rcache: &mut LfsCache,
    hint: lfs_size_t,
    block: lfs_block_t,
    off: lfs_off_t,
    buffer: &mut [u8],
) -> Result<(), Error> {
    let cfg = unsafe { lfs.cfg.as_ref() };

    if off + (buffer.len() as u32) > cfg.block_size
        || (lfs.block_count != 0 && block >= lfs.block_count)
    {
        return crate::lfs_err!(Err(Error::Corrupt));
    }

    let mut data = buffer;
    let mut off = off;
    let mut size = data.len() as u32;

    while size > 0 {
        let mut diff = size;

        if let Some(pcache) = pcache
            && block == pcache.block
            && off < pcache.off + pcache.size
        {
            if off >= pcache.off {
                diff = core::cmp::min(diff, pcache.size - (off - pcache.off));
                data[..diff as usize].copy_from_slice(unsafe {
                    &pcache.buffer.as_ref()
                        [((off - pcache.off) as usize)..((off - pcache.off + diff) as usize)]
                });

                data = &mut data[(diff as usize)..];
                off += diff;
                size -= diff;
                continue;
            }
            diff = diff.min(pcache.off - off);
        }

        if block == rcache.block && off < rcache.off + rcache.size {
            if off >= rcache.off {
                diff = cmp::min(diff, rcache.size - (off - rcache.off));
                data[..diff as usize].copy_from_slice(unsafe {
                    &rcache.buffer.as_ref()
                        [((off - rcache.off) as usize)..((off - rcache.off + diff) as usize)]
                });

                data = &mut data[(diff as usize)..];
                off += diff;
                size -= diff;
                continue;
            }
            diff = cmp::min(diff, rcache.off - off);
        }

        if size >= hint && off.is_multiple_of(cfg.read_size) && size >= cfg.read_size {
            diff = lfs_aligndown(diff, cfg.read_size);
            crate::lfs_trace!("bd_read block={} off={} size={}", block, off, diff);
            let data_ = data.split_at_mut(diff as _);
            lfs_pass_err!(
                unsafe { lfs.cfg.as_ref().context.unwrap().as_mut() }.read(block, off, data_.0),
                "bd_read block={} -> CORRUPT",
                block
            )?;

            data = data_.1;
            off += diff;
            size -= diff;
            continue;
        }

        crate::lfs_assert!(lfs.block_count == 0 || block < lfs.block_count);
        rcache.block = block;
        rcache.off = lfs_aligndown(off, cfg.read_size);
        rcache.size = cmp::min(
            cmp::min(lfs_alignup(off + hint, cfg.read_size), cfg.block_size)
                .saturating_sub(rcache.off),
            rcache.buffer.len() as u32,
        );
        crate::lfs_trace!(
            "bd_read block={} off={} size={}",
            rcache.block,
            rcache.off,
            rcache.size
        );
        let data_ = unsafe { &mut rcache.buffer.as_mut()[..rcache.size as usize] };
        let err = unsafe { lfs.cfg.as_ref().context.unwrap().as_mut() }.read(
            rcache.block,
            rcache.off,
            data_,
        );
        if err.is_err() {
            crate::lfs_trace!("bd_read block={} -> CORRUPT", rcache.block);
            // Don't leave rcache claiming to have this block when the buffer wasn't filled.
            // A retry (e.g. after bad-block clear) would otherwise serve stale data.
            rcache.block = crate::types::LFS_BLOCK_NULL;
            return crate::lfs_pass_err!(err);
        }
    }

    Ok(())
}

/// Per lfs.c lfs_bd_cmp (lines 128-154)
///
/// C:
/// ```c
/// static int lfs_bd_cmp(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache, lfs_size_t hint,
///         lfs_block_t block, lfs_off_t off,
///         const void *buffer, lfs_size_t size) {
///     const uint8_t *data = buffer;
///     lfs_size_t diff = 0;
///
///     for (lfs_off_t i = 0; i < size; i += diff) {
///         uint8_t dat[8];
///
///         diff = lfs_min(size-i, sizeof(dat));
///         int err = lfs_bd_read(lfs,
///                 pcache, rcache, hint-i,
///                 block, off+i, &dat, diff);
///         if (err) {
///             return err;
///         }
///
///         int res = memcmp(dat, data + i, diff);
///         if (res) {
///             return res < 0 ? LFS_CMP_LT : LFS_CMP_GT;
///         }
///     }
///
///     return LFS_CMP_EQ;
/// }
/// ```
pub fn lfs_bd_cmp(
    lfs: &Lfs,
    pcache: Option<&LfsCache>,
    rcache: &mut LfsCache,
    hint: lfs_size_t,
    block: lfs_block_t,
    off: lfs_off_t,
    mut buffer: &[u8],
) -> Result<core::cmp::Ordering, Error> {
    let mut i: lfs_off_t = 0;

    while !buffer.is_empty() {
        let mut dat = [0u8; 8];
        let diff = core::cmp::min(buffer.len(), 8);
        lfs_bd_read(
            lfs,
            pcache,
            rcache,
            hint - i,
            block,
            off + i,
            &mut dat[..diff],
        )?;

        let res = {
            let disk = &dat[..diff];
            let expected = &buffer[..diff];
            disk.cmp(expected)
        };
        match res {
            core::cmp::Ordering::Equal => {}
            core::cmp::Ordering::Less => return Ok(core::cmp::Ordering::Less),
            core::cmp::Ordering::Greater => return Ok(core::cmp::Ordering::Greater),
        }
        i += diff as lfs_off_t;
        buffer = &buffer[diff..];
    }
    Ok(core::cmp::Ordering::Equal)
}

/// Per lfs.c lfs_bd_crc (lines 155-175)
///
/// C:
/// ```c
/// static int lfs_bd_crc(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache, lfs_size_t hint,
///         lfs_block_t block, lfs_off_t off, lfs_size_t size, uint32_t *crc) {
///     lfs_size_t diff = 0;
///
///     for (lfs_off_t i = 0; i < size; i += diff) {
///         uint8_t dat[8];
///         diff = lfs_min(size-i, sizeof(dat));
///         int err = lfs_bd_read(lfs,
///                 pcache, rcache, hint-i,
///                 block, off+i, &dat, diff);
///         if (err) {
///             return err;
///         }
///
///         *crc = lfs_crc(*crc, &dat, diff);
///     }
///
///     return 0;
/// }
/// ```
pub fn lfs_bd_crc(
    lfs: &Lfs,
    pcache: Option<&LfsCache>,
    rcache: &mut LfsCache,
    hint: lfs_size_t,
    block: lfs_block_t,
    off: lfs_off_t,
    size: lfs_size_t,
    crc: &mut u32,
) -> Result<(), Error> {
    use crate::crc::lfs_crc;

    let mut i: lfs_off_t = 0;
    while i < size {
        let mut dat = [0u8; 8];
        let diff = core::cmp::min(size - i, 8) as usize;
        lfs_bd_read(
            lfs,
            pcache,
            rcache,
            hint.saturating_sub(i),
            block,
            off + i,
            &mut dat[0..diff],
        )?;

        *crc = lfs_crc(*crc, &dat[..(diff as usize)]);

        i += diff as lfs_off_t;
    }
    Ok(())
}

/// Per lfs.c lfs_bd_flush (lines 177-210)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_flush(lfs_t *lfs,
///         lfs_cache_t *pcache, lfs_cache_t *rcache, bool validate) {
///     if (pcache->block != LFS_BLOCK_NULL && pcache->block != LFS_BLOCK_INLINE) {
///         LFS_ASSERT(pcache->block < lfs->block_count);
///         lfs_size_t diff = lfs_alignup(pcache->size, lfs->cfg->prog_size);
///         int err = lfs->cfg->prog(lfs->cfg, pcache->block,
///                 pcache->off, pcache->buffer, diff);
///         LFS_ASSERT(err <= 0);
///         if (err) {
///             return err;
///         }
///
///         if (validate) {
///             // check data on disk
///             lfs_cache_drop(lfs, rcache);
///             int res = lfs_bd_cmp(lfs,
///                     NULL, rcache, diff,
///                     pcache->block, pcache->off, pcache->buffer, diff);
///             if (res < 0) {
///                 return res;
///             }
///
///             if (res != LFS_CMP_EQ) {
///                 return LFS_ERR_CORRUPT;
///             }
///         }
///
///         lfs_cache_zero(lfs, pcache);
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_bd_flush(
    lfs: &Lfs,
    pcache: &mut LfsCache,
    rcache: &mut LfsCache,
    validate: bool,
) -> Result<(), Error> {
    use crate::types::LFS_BLOCK_INLINE;
    use crate::util::lfs_alignup;

    let cfg = unsafe { lfs.cfg.as_ref() };

    if pcache.block != crate::types::LFS_BLOCK_NULL && pcache.block != LFS_BLOCK_INLINE {
        crate::lfs_assert!(pcache.block < lfs.block_count);
        let diff = lfs_alignup(pcache.size, cfg.prog_size) as usize;
        crate::lfs_trace!(
            "bd_prog block={} off={} size={}",
            pcache.block,
            pcache.off,
            diff
        );
        let data_ = unsafe { &pcache.buffer.as_ref()[..diff] };
        let err = unsafe { lfs.cfg.as_ref().context.unwrap().as_mut() }.write(
            pcache.block,
            pcache.off,
            data_,
        );
        crate::lfs_pass_err!(err, "bd_prog block={} -> CORRUPT", pcache.block)?;

        if validate {
            lfs_cache_drop(lfs, rcache);
            let res = lfs_bd_cmp(
                lfs,
                None,
                rcache,
                diff as u32,
                pcache.block,
                pcache.off,
                data_,
            );
            res?;
            if let Ok(res) = res
                && res != core::cmp::Ordering::Equal
            {
                return crate::lfs_err!(Err(Error::Corrupt));
            }
        }

        lfs_cache_zero(lfs, pcache);
    }

    Ok(())
}

/// Per lfs.c lfs_bd_sync (lines 213-226)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_sync(lfs_t *lfs,
///         lfs_cache_t *pcache, lfs_cache_t *rcache, bool validate) {
///     lfs_cache_drop(lfs, rcache);
///
///     int err = lfs_bd_flush(lfs, pcache, rcache, validate);
///     if (err) {
///         return err;
///     }
///
///     err = lfs->cfg->sync(lfs->cfg);
///     LFS_ASSERT(err <= 0);
///     return err;
/// }
/// #endif
/// ```
pub fn lfs_bd_sync(
    lfs: &Lfs,
    pcache: &mut LfsCache,
    rcache: &mut LfsCache,
    validate: bool,
) -> Result<(), Error> {
    lfs_cache_drop(lfs, rcache);

    lfs_bd_flush(lfs, pcache, rcache, validate)?;

    unsafe { lfs.cfg.as_ref().context.unwrap().as_mut() }.sync()
}

/// Per lfs.c lfs_bd_prog (lines 228-274)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_prog(lfs_t *lfs,
///         lfs_cache_t *pcache, lfs_cache_t *rcache, bool validate,
///         lfs_block_t block, lfs_off_t off,
///         const void *buffer, lfs_size_t size) {
///     const uint8_t *data = buffer;
///     LFS_ASSERT(block == LFS_BLOCK_INLINE || block < lfs->block_count);
///     LFS_ASSERT(off + size <= lfs->cfg->block_size);
///
///     while (size > 0) {
///         if (block == pcache->block &&
///                 off >= pcache->off &&
///                 off < pcache->off + lfs->cfg->cache_size) {
///             // already fits in pcache?
///             lfs_size_t diff = lfs_min(size,
///                     lfs->cfg->cache_size - (off-pcache->off));
///             memcpy(&pcache->buffer[off-pcache->off], data, diff);
///
///             data += diff;
///             off += diff;
///             size -= diff;
///
///             pcache->size = lfs_max(pcache->size, off - pcache->off);
///             if (pcache->size == lfs->cfg->cache_size) {
///                 // eagerly flush out pcache if we fill up
///                 int err = lfs_bd_flush(lfs, pcache, rcache, validate);
///                 if (err) {
///                     return err;
///                 }
///             }
///
///             continue;
///         }
///
///         // pcache must have been flushed, either by programming and
///         // entire block or manually flushing the pcache
///         LFS_ASSERT(pcache->block == LFS_BLOCK_NULL);
///
///         // prepare pcache, first condition can no longer fail
///         pcache->block = block;
///         pcache->off = lfs_aligndown(off, lfs->cfg->prog_size);
///         pcache->size = 0;
///     }
///
///     return 0;
/// }
/// #endif
/// ```
pub fn lfs_bd_prog(
    lfs: &Lfs,
    pcache: &mut LfsCache,
    rcache: &mut LfsCache,
    validate: bool,
    block: lfs_block_t,
    off: lfs_off_t,
    buffer: &[u8],
) -> Result<(), Error> {
    use crate::types::LFS_BLOCK_INLINE;
    use crate::util::{lfs_aligndown, lfs_min};

    let cfg = unsafe { lfs.cfg.as_ref() };

    crate::lfs_assert!(block == LFS_BLOCK_INLINE || block < lfs.block_count);
    crate::lfs_assert!(off + buffer.len() as u32 <= cfg.block_size);

    let mut data = buffer;
    let mut off = off;
    let mut size = buffer.len() as u32;

    while size > 0 {
        if block == pcache.block
            && off >= pcache.off
            && off < pcache.off + pcache.buffer.len() as u32
        {
            let diff = lfs_min(size, pcache.buffer.len() as u32 - (off - pcache.off));
            // Trace superblock magic region (offset 12-20 in block 0/1)
            if (block == 0 || block == 1) && off <= 12 && off + diff > 12 {
                let magic_start = 12usize.saturating_sub(off as usize);
                let magic_len = core::cmp::min(8, diff as usize - magic_start);
                if magic_len > 0 {
                    crate::lfs_trace!(
                        "bd_prog superblock block={} off={} size={} magic_region[{}..{}]={:?}",
                        block,
                        off,
                        size,
                        magic_start,
                        magic_start + magic_len,
                        unsafe {
                            core::slice::from_raw_parts(data.as_ptr().add(magic_start), magic_len)
                        }
                    );
                }
            }
            unsafe {
                pcache.buffer.as_mut()
                    [((off - pcache.off) as usize)..(off - pcache.off + diff) as usize]
                    .copy_from_slice(&data[..diff as usize]);
            };

            data = &data[(diff as usize)..];
            off += diff;
            size -= diff;

            pcache.size = cmp::max(pcache.size, off - pcache.off);
            if pcache.size == pcache.buffer.len() as u32 {
                lfs_bd_flush(lfs, pcache, rcache, validate)?;
            }

            continue;
        }

        crate::lfs_assert!(pcache.block == crate::types::LFS_BLOCK_NULL);

        pcache.block = block;
        pcache.off = lfs_aligndown(off, cfg.prog_size);
        pcache.size = 0;
    }

    Ok(())
}

/// Per lfs.c lfs_bd_erase (lines 277-282)
///
/// C:
/// ```c
/// #ifndef LFS_READONLY
/// static int lfs_bd_erase(lfs_t *lfs, lfs_block_t block) {
///     LFS_ASSERT(block < lfs->block_count);
///     int err = lfs->cfg->erase(lfs->cfg, block);
///     LFS_ASSERT(err <= 0);
///     return err;
/// }
/// #endif
/// ```
pub fn lfs_bd_erase(lfs: &Lfs, block: lfs_block_t) -> Result<(), Error> {
    crate::lfs_assert!(block < lfs.block_count);
    crate::lfs_trace!("bd_erase block={}", block);
    let err = unsafe { lfs.cfg.as_ref().context.unwrap().as_mut() }.erase(block);
    if err.is_err() {
        crate::lfs_trace!("bd_erase block={} -> CORRUPT", block);
    }
    err
}
