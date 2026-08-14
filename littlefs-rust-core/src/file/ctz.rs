//! CTZ operations. Per lfs.c lfs_ctz_index, lfs_ctz_find, lfs_ctz_extend, lfs_ctz_traverse.

use zerocopy::IntoBytes;

use crate::Lfs;
use crate::error::Error;
use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t};

#[repr(C)]
pub struct LfsCtz {
    pub head: lfs_block_t,
    pub size: lfs_size_t,
}

/// Per lfs.c lfs_ctz_fromle32 (lines 475-479)
///
/// C:
/// ```c
/// static void lfs_ctz_fromle32(struct lfs_ctz *ctz) {
///     ctz->head = lfs_fromle32(ctz->head);
///     ctz->size = lfs_fromle32(ctz->size);
/// }
/// ```
pub fn lfs_ctz_fromle32(ctz: *mut LfsCtz) {
    if ctz.is_null() {
        return;
    }
    unsafe {
        let ctz = &mut *ctz;
        ctz.head = crate::util::lfs_fromle32(ctz.head);
        ctz.size = crate::util::lfs_fromle32(ctz.size);
    }
}

/// Per lfs.c lfs_ctz_tole32 (lines 481-486)
///
/// C:
/// ```c
/// static void lfs_ctz_tole32(struct lfs_ctz *ctz) {
///     ctz->head = lfs_tole32(ctz->head);
///     ctz->size = lfs_tole32(ctz->size);
/// }
/// #endif
/// ```
pub fn lfs_ctz_tole32(ctz: *mut LfsCtz) {
    if ctz.is_null() {
        return;
    }
    unsafe {
        let ctz = &mut *ctz;
        ctz.head = crate::util::lfs_tole32(ctz.head);
        ctz.size = crate::util::lfs_tole32(ctz.size);
    }
}

/// Per lfs.c lfs_ctz_index (lines 2873-2884)
///
/// C:
/// ```c
/// static int lfs_ctz_index(lfs_t *lfs, lfs_off_t *off) {
///     lfs_off_t size = *off;
///     lfs_off_t b = lfs->cfg->block_size - 2*4;
///     lfs_off_t i = size / b;
///     if (i == 0) {
///         return 0;
///     }
///
///     i = (size - 4*(lfs_popc(i-1)+2)) / b;
///     *off = size - b*i - 4*lfs_popc(i);
///     return i;
/// }
/// ```
pub fn lfs_ctz_index(lfs: &crate::fs::Lfs, off: *mut lfs_off_t) -> i32 {
    use crate::util::lfs_popc;

    if off.is_null() {
        return 0;
    }
    unsafe {
        let size = *off;
        let block_size = lfs.cfg.as_ref().expect("cfg").block_size;
        let b = block_size - 8;
        let mut i = size / b;
        if i == 0 {
            return 0;
        }
        i = (size - 4 * (lfs_popc(i - 1) + 2)) / b;
        *off = size - b * i - 4 * lfs_popc(i);
        i as i32
    }
}

/// Per lfs.c lfs_ctz_find (lines 2886-2919)
///
/// Translation docs: Traverses the CTZ skip-list from head to find the block
/// containing the given file position. Returns block index and offset.
///
/// C:
/// ```c
/// static int lfs_ctz_find(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache,
///         lfs_block_t head, lfs_size_t size,
///         lfs_size_t pos, lfs_block_t *block, lfs_off_t *off) {
///     if (size == 0) {
///         *block = LFS_BLOCK_NULL;
///         *off = 0;
///         return 0;
///     }
///
///     lfs_off_t current = lfs_ctz_index(lfs, &(lfs_off_t){size-1});
///     lfs_off_t target = lfs_ctz_index(lfs, &pos);
///
///     while (current > target) {
///         lfs_size_t skip = lfs_min(
///                 lfs_npw2(current-target+1) - 1,
///                 lfs_ctz(current));
///
///         int err = lfs_bd_read(lfs,
///                 pcache, rcache, sizeof(head),
///                 head, 4*skip, &head, sizeof(head));
///         head = lfs_fromle32(head);
///         if (err) {
///             return err;
///         }
///
///         current -= 1 << skip;
///     }
///
///     *block = head;
///     *off = pos;
///     return 0;
/// }
/// ```
pub fn lfs_ctz_find(
    lfs: &Lfs,
    pcache: Option<&crate::bd::LfsCache>,
    rcache: &mut crate::bd::LfsCache,
    head: lfs_block_t,
    size: lfs_size_t,
    pos: lfs_size_t,
    block: &mut lfs_block_t,
    off: &mut lfs_off_t,
) -> Result<(), Error> {
    use crate::bd::bd::lfs_bd_read;
    use crate::types::LFS_BLOCK_NULL;
    use crate::util::{lfs_ctz, lfs_fromle32, lfs_min, lfs_npw2};

    if size == 0 {
        *block = LFS_BLOCK_NULL;
        *off = 0;

        return Ok(());
    }

    unsafe {
        let block_size = lfs.cfg.as_ref().expect("cfg").block_size;
        let mut current_off = size - 1;
        let mut target_off = pos;
        let mut current = lfs_ctz_index(lfs, &mut current_off);
        let target = lfs_ctz_index(lfs, &mut target_off);

        let mut head_val = head;
        #[cfg(feature = "loop_limits")]
        const MAX_CTZ_FIND_ITER: u32 = 4096;
        #[cfg(feature = "loop_limits")]
        let mut iter: u32 = 0;

        while current > target {
            #[cfg(feature = "loop_limits")]
            {
                iter += 1;
                if iter > MAX_CTZ_FIND_ITER {
                    panic!(
                        "loop_limits: MAX_CTZ_FIND_ITER ({}) exceeded",
                        MAX_CTZ_FIND_ITER
                    );
                }
            }
            let skip = lfs_min(
                lfs_npw2((current - target + 1) as u32) - 1,
                lfs_ctz(current as u32),
            );

            let mut head_buf: u32 = 0;
            lfs_bd_read(
                lfs,
                pcache,
                rcache,
                4,
                head_val,
                4 * skip,
                head_buf.as_mut_bytes(),
            )?;

            head_val = lfs_fromle32(head_buf);

            current -= 1 << skip;
        }

        *block = head_val;
        *off = target_off;
    }
    Ok(())
}

/// Per lfs.c lfs_ctz_traverse (lines 3020-3063)
///
/// C:
/// ```c
/// static int lfs_ctz_traverse(lfs_t *lfs,
///         const lfs_cache_t *pcache, lfs_cache_t *rcache,
///         lfs_block_t head, lfs_size_t size,
///         int (*cb)(void*, lfs_block_t), void *data) {
///     if (size == 0) {
///         return 0;
///     }
///
///     lfs_off_t index = lfs_ctz_index(lfs, &(lfs_off_t){size-1});
///
///     while (true) {
///         int err = cb(data, head);
///         if (err) {
///             return err;
///         }
///
///         if (index == 0) {
///             return 0;
///         }
///
///         lfs_block_t heads[2];
///         int count = 2 - (index & 1);
///         err = lfs_bd_read(lfs,
///                 pcache, rcache, count*sizeof(head),
///                 head, 0, &heads, count*sizeof(head));
///         heads[0] = lfs_fromle32(heads[0]);
///         heads[1] = lfs_fromle32(heads[1]);
///         if (err) {
///             return err;
///         }
///
///         for (int i = 0; i < count-1; i++) {
///             err = cb(data, heads[i]);
///             if (err) {
///                 return err;
///             }
///         }
///
///         head = heads[count-1];
///         index -= count;
///     }
/// }
/// ```
#[allow(clippy::type_complexity)]
pub fn lfs_ctz_traverse(
    lfs: &crate::fs::Lfs,
    pcache: Option<&crate::bd::LfsCache>,
    rcache: &mut crate::bd::LfsCache,
    head: lfs_block_t,
    size: lfs_size_t,
    cb: fn(*mut core::ffi::c_void, lfs_block_t) -> Result<(), Error>,
    data: *mut core::ffi::c_void,
) -> Result<(), Error> {
    use crate::bd::bd::lfs_bd_read;
    use crate::util::lfs_fromle32;

    if size == 0 {
        return Ok(());
    }

    let mut index_off = size - 1;
    let mut index = lfs_ctz_index(lfs, &mut index_off) as u32;
    let mut current_head = head;
    #[cfg(feature = "loop_limits")]
    let block_count = lfs.block_count;
    #[cfg(feature = "loop_limits")]
    let mut iter: u32 = 0;

    loop {
        #[cfg(feature = "loop_limits")]
        {
            if iter >= block_count {
                panic!(
                    "loop_limits: lfs_ctz_traverse iter ({}) >= block_count ({})",
                    iter, block_count
                );
            }
            iter += 1;
        }
        cb(data, current_head)?;

        if index == 0 {
            return Ok(());
        }

        // C: count*sizeof(head) as hint
        let count = (2 - (index & 1)) as usize;
        let mut heads = [0u32; 2];
        let read_size = (count * core::mem::size_of::<lfs_block_t>()) as u32;
        lfs_bd_read(
            lfs,
            pcache,
            rcache,
            read_size,
            current_head,
            0,
            heads.as_mut_bytes(),
        )?;

        heads[0] = lfs_fromle32(heads[0]);
        heads[1] = lfs_fromle32(heads[1]);

        #[allow(clippy::needless_range_loop)] // Rule 2: preserve C loop structure
        for i in 0..count - 1 {
            cb(data, heads[i])?;
        }

        current_head = heads[count - 1];
        index = index.wrapping_sub(count as u32);
    }
}

/// Translation docs: Extends a CTZ file by allocating a new block. For size 0 returns
/// the new block; for partial last block copies bytes; for full block appends skip-list.
/// Retries on LFS_ERR_CORRUPT after cache drop.
///
/// Per lfs.c lfs_ctz_extend (lines 2921-3018)
///
/// C:
/// ```c
/// static int lfs_ctz_extend(lfs_t *lfs,
///         lfs_cache_t *pcache, lfs_cache_t *rcache,
///         lfs_block_t head, lfs_size_t size,
///         lfs_block_t *block, lfs_off_t *off) {
///     while (true) {
///         // go ahead and grab a block
///         lfs_block_t nblock;
///         int err = lfs_alloc(lfs, &nblock);
///         if (err) {
///             return err;
///         }
///
///         {
///             err = lfs_bd_erase(lfs, nblock);
///             if (err) {
///                 if (err == LFS_ERR_CORRUPT) {
///                     goto relocate;
///                 }
///                 return err;
///             }
///
///             if (size == 0) {
///                 *block = nblock;
///                 *off = 0;
///                 return 0;
///             }
///
///             lfs_size_t noff = size - 1;
///             lfs_off_t index = lfs_ctz_index(lfs, &noff);
///             noff = noff + 1;
///
///             // just copy out the last block if it is incomplete
///             if (noff != lfs->cfg->block_size) {
///                 for (lfs_off_t i = 0; i < noff; i++) {
///                     uint8_t data;
///                     err = lfs_bd_read(lfs,
///                             NULL, rcache, noff-i,
///                             head, i, &data, 1);
///                     if (err) {
///                         return err;
///                     }
///
///                     err = lfs_bd_prog(lfs,
///                             pcache, rcache, true,
///                             nblock, i, &data, 1);
///                     if (err) {
///                         if (err == LFS_ERR_CORRUPT) {
///                             goto relocate;
///                         }
///                         return err;
///                     }
///                 }
///
///                 *block = nblock;
///                 *off = noff;
///                 return 0;
///             }
///
///             // append block
///             index += 1;
///             lfs_size_t skips = lfs_ctz(index) + 1;
///             lfs_block_t nhead = head;
///             for (lfs_off_t i = 0; i < skips; i++) {
///                 nhead = lfs_tole32(nhead);
///                 err = lfs_bd_prog(lfs, pcache, rcache, true,
///                         nblock, 4*i, &nhead, 4);
///                 nhead = lfs_fromle32(nhead);
///                 if (err) {
///                     if (err == LFS_ERR_CORRUPT) {
///                         goto relocate;
///                     }
///                     return err;
///                 }
///
///                 if (i != skips-1) {
///                     err = lfs_bd_read(lfs,
///                             NULL, rcache, sizeof(nhead),
///                             nhead, 4*i, &nhead, sizeof(nhead));
///                     nhead = lfs_fromle32(nhead);
///                     if (err) {
///                         return err;
///                     }
///                 }
///             }
///
///             *block = nblock;
///             *off = 4*skips;
///             return 0;
///         }
///
/// relocate:
///         LFS_DEBUG("Bad block at 0x%"PRIx32, nblock);
///
///         // just clear cache and try a new block
///         lfs_cache_drop(lfs, pcache);
///     }
/// }
/// #endif
/// ```
pub fn lfs_ctz_extend(
    lfs: &mut crate::fs::Lfs,
    pcache: &mut crate::bd::LfsCache,
    rcache: &mut crate::bd::LfsCache,
    head: lfs_block_t,
    size: lfs_size_t,
    block: &mut lfs_block_t,
    off: &mut lfs_off_t,
) -> Result<(), Error> {
    use crate::bd::bd::{lfs_bd_erase, lfs_bd_prog, lfs_bd_read, lfs_cache_drop};
    use crate::block_alloc::alloc::{lfs_alloc, lfs_alloc_lookahead};
    use crate::util::{lfs_ctz, lfs_fromle32, lfs_tole32};

    'relocate: loop {
        unsafe {
            let block_size = lfs.cfg.as_ref().expect("cfg").block_size;

            let mut nblock: lfs_block_t = 0;
            lfs_alloc(lfs, &mut nblock)?;

            let err = lfs_bd_erase(lfs, nblock);
            if let Err(err) = err {
                if err == Error::Corrupt {
                    lfs_alloc_lookahead(lfs, nblock);
                    lfs_cache_drop(lfs, pcache);
                    continue 'relocate;
                }
                return crate::lfs_pass_err!(Err(err));
            }

            if size == 0 {
                *block = nblock;
                *off = 0;
                return Ok(());
            }

            let mut noff = size - 1;
            let mut index = lfs_ctz_index(lfs, &mut noff);
            noff += 1;

            if noff != block_size {
                for i in 0..noff {
                    let mut data: u8 = 0;
                    lfs_bd_read(lfs, None, rcache, noff - i, head, i, data.as_mut_bytes())?;

                    let err = lfs_bd_prog(lfs, pcache, rcache, true, nblock, i, data.as_bytes());
                    if let Err(err) = err {
                        if err == Error::Corrupt {
                            lfs_alloc_lookahead(lfs, nblock);
                            lfs_cache_drop(lfs, pcache);
                            continue 'relocate;
                        }
                        return crate::lfs_pass_err!(Err(err));
                    }
                }
                *block = nblock;
                *off = noff;
                return Ok(());
            }

            index += 1;
            let skips = lfs_ctz(index as u32) + 1;
            let mut nhead = head;
            for i in 0..skips {
                let nhead_le = lfs_tole32(nhead);
                let err = lfs_bd_prog(
                    lfs,
                    pcache,
                    rcache,
                    true,
                    nblock,
                    4 * i,
                    nhead_le.as_bytes(),
                );
                if let Err(err) = err {
                    if err == Error::Corrupt {
                        let _ = lfs_alloc_lookahead(lfs, nblock);
                        lfs_cache_drop(lfs, pcache);
                        continue 'relocate;
                    }
                    return crate::lfs_pass_err!(Err(err));
                }
                nhead = lfs_fromle32(nhead_le);

                if i != skips - 1 {
                    let mut nhead_buf: u32 = 0;

                    lfs_bd_read(lfs, None, rcache, 4, nhead, 4 * i, nhead_buf.as_mut_bytes())?;

                    nhead = lfs_fromle32(nhead_buf);
                }
            }

            *block = nblock;
            *off = 4 * skips;
            return Ok(());
        }
    }
}
