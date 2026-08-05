//! Global state. Per lfs.h lfs_gstate_t and lfs.c lfs_gstate_*.

use crate::tag::Tag;
use crate::types::{lfs_block_t, lfs_tag_t};
use crate::util::lfs_pair_cmp;
use crate::util::{lfs_fromle32, lfs_tole32};

/// Per lfs.h typedef struct lfs_gstate (lines 429-432)
///
/// C:
/// ```c
/// typedef struct lfs_gstate {
///     uint32_t tag;
///     lfs_block_t pair[2];
/// } lfs_gstate_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LfsGstateT<T> {
    pub tag: T,
    pub pair: [lfs_block_t; 2],
}

pub type LfsGstate = LfsGstateT<Tag>;

/// Per lfs.c lfs_gstate_xor (lines 407-412)
///
/// C:
/// ```c
/// static inline void lfs_gstate_xor(lfs_gstate_t *a, const lfs_gstate_t *b) {
///     a->tag ^= b->tag;
///     a->pair[0] ^= b->pair[0];
///     a->pair[1] ^= b->pair[1];
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_xor(a: &mut LfsGstate, b: &LfsGstate) {
    a.tag.0 ^= b.tag.0;
    a.pair[0] ^= b.pair[0];
    a.pair[1] ^= b.pair[1];
}

/// Per lfs.c lfs_gstate_iszero (lines 413-418)
///
/// C:
/// ```c
/// static inline bool lfs_gstate_iszero(const lfs_gstate_t *a) {
///     return a->tag == 0 && a->pair[0] == 0 && a->pair[1] == 0;
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_iszero(a: &LfsGstate) -> bool {
    a.tag.0 == 0 && a.pair[0] == 0 && a.pair[1] == 0
}

/// Per lfs.c lfs_gstate_hasorphans (lines 420-422)
///
/// C:
/// ```c
/// static inline bool lfs_gstate_hasorphans(const lfs_gstate_t *a) {
///     return lfs_tag_size(a->tag);
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_hasorphans(a: &LfsGstate) -> bool {
    a.tag.size() != 0
}

/// Per lfs.c lfs_gstate_getorphans (lines 424-426)
///
/// C:
/// ```c
/// static inline uint8_t lfs_gstate_getorphans(const lfs_gstate_t *a) {
///     return lfs_tag_size(a->tag) & 0x1ff;
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_getorphans(a: &LfsGstate) -> u8 {
    (a.tag.size() & 0x1ff) as u8
}

/// Per lfs.c lfs_gstate_hasmove (lines 428-430)
///
/// C:
/// ```c
/// static inline bool lfs_gstate_hasmove(const lfs_gstate_t *a) {
///     return lfs_tag_type1(a->tag);
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_hasmove(a: &LfsGstate) -> bool {
    a.tag.lfs_tag_type1() != 0
}

/// Per lfs.c lfs_gstate_needssuperblock (lines 433-435)
///
/// C:
/// ```c
/// static inline bool lfs_gstate_needssuperblock(const lfs_gstate_t *a) {
///     return lfs_tag_size(a->tag) >> 9;
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_needssuperblock(a: &LfsGstate) -> bool {
    (a.tag.size() >> 9) != 0
}

/// Per lfs.c lfs_gstate_hasmovehere (lines 437-440)
///
/// C:
/// ```c
/// static inline bool lfs_gstate_hasmovehere(const lfs_gstate_t *a,
///         const lfs_block_t *pair) {
///     return lfs_tag_type1(a->tag) && lfs_pair_cmp(a->pair, pair) == 0;
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_hasmovehere(a: &LfsGstate, pair: &[lfs_block_t; 2]) -> bool {
    a.tag.lfs_tag_type1() != 0 && lfs_pair_cmp(&a.pair, pair) == 0
}

/// Per lfs.c lfs_gstate_fromle32 (lines 442-446)
///
/// C:
/// ```c
/// static inline void lfs_gstate_fromle32(lfs_gstate_t *a) {
///     a->tag     = lfs_fromle32(a->tag);
///     a->pair[0] = lfs_fromle32(a->pair[0]);
///     a->pair[1] = lfs_fromle32(a->pair[1]);
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_fromle32(a: &LfsGstateT<u32>) -> LfsGstate {
    LfsGstate {
        tag: Tag(u32::from_le(a.tag)),
        pair: [u32::from_le(a.pair[0]), u32::from_le(a.pair[1])],
    }
}

/// Per lfs.c lfs_gstate_tole32 (lines 449-453)
///
/// C:
/// ```c
/// static inline void lfs_gstate_tole32(lfs_gstate_t *a) {
///     a->tag     = lfs_tole32(a->tag);
///     a->pair[0] = lfs_tole32(a->pair[0]);
///     a->pair[1] = lfs_tole32(a->pair[1]);
/// }
/// ```
#[inline(always)]
pub fn lfs_gstate_tole32(a: &LfsGstate) -> LfsGstateT<u32> {
    LfsGstateT::<u32> {
        tag: a.tag.0.to_le(),
        pair: [a.pair[0].to_le(), a.pair[1].to_le()],
    }
}
