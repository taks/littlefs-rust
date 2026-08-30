//! Open list node. Per lfs.h struct lfs_mlist.

use core::fmt::Debug;

use super::lfs_mdir::LfsMdir;

/// Per lfs.h struct lfs_mlist
#[repr(C)]
pub struct LfsMlist {
    pub next: *mut LfsMlist,
    pub id: u16,
    pub type_: u8,
    pub m: LfsMdir,
}

impl Debug for LfsMlist {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LfsMlist")
            .field("next", &self.next)
            .field("id", &self.id)
            .field("type_", &self.type_)
            .field("m", &self.m)
            .finish()
    }
}

/// Per lfs.c lfs_mlist_isopen (lines 508-518)
///
/// C:
/// ```c
/// static bool lfs_mlist_isopen(struct lfs_mlist *head,
///         struct lfs_mlist *node) {
///     for (struct lfs_mlist **p = &head; *p; p = &(*p)->next) {
///         if (*p == (struct lfs_mlist*)node) {
///             return true;
///         }
///     }
///
///     return false;
/// }
/// ```
#[allow(unused)]
pub fn lfs_mlist_isopen(head: *mut LfsMlist, node: *const LfsMlist) -> bool {
    unsafe {
        let mut p = head;
        while !p.is_null() {
            if core::ptr::eq(p, node) {
                return true;
            }
            p = (*p).next;
        }
        false
    }
}

/// Per lfs.c lfs_mlist_remove (lines 520-527)
///
/// C:
/// ```c
/// static void lfs_mlist_remove(lfs_t *lfs, struct lfs_mlist *mlist) {
///     for (struct lfs_mlist **p = &lfs->mlist; *p; p = &(*p)->next) {
///         if (*p == mlist) {
///             *p = (*p)->next;
///             break;
///         }
///     }
/// }
/// ```
pub fn lfs_mlist_remove<S>(lfs: &mut crate::fs::Lfs<S>, mlist: &mut LfsMlist) {
    unsafe {
        let mut p = &mut lfs.mlist;
        while !(*p).is_null() {
            if core::ptr::eq(*p, mlist) {
                *p = mlist.next;
                break;
            }
            p = &mut (*(*p)).next;
        }
    }
}

/// Per lfs.c lfs_mlist_append (lines 529-533)
///
/// C:
/// ```c
/// static void lfs_mlist_append(lfs_t *lfs, struct lfs_mlist *mlist) {
///     mlist->next = lfs->mlist;
///     lfs->mlist = mlist;
/// }
/// ```
pub fn lfs_mlist_append<S>(lfs: &mut crate::fs::Lfs<S>, mlist: &mut LfsMlist) {
    let head = lfs.mlist;
    mlist.next = head;
    lfs.mlist = mlist;
}
