//! Hand-translated LittleFS from C to Rust.
//!
//! Logic and architecture kept close to reference/lfs.c. Uses `unsafe` where needed.
//! Safe wrapper API deferred until core passes all tests.

#![no_std]
#![allow(clippy::too_many_arguments)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod bd;
mod block_alloc;
mod crc;
mod dir;
#[cfg(feature = "alloc")]
mod lfs_alloc_module;

pub mod error;
mod file;
mod fs;
mod lfs_config;
mod lfs_gstate;
mod lfs_info;
mod lfs_superblock;
pub mod lfs_type;
#[cfg(test)]
mod test;
#[macro_use]
mod macros;
mod borrow_unchecked;
mod tag;
mod types;
mod util;

pub use crate::dir::LfsDir;
use crate::error::Error;
pub use crate::file::LfsFile;
pub use crate::fs::Lfs;
pub use crate::lfs_config::{LfsConfig, Storage};
pub use crate::lfs_info::{LfsAttr, LfsFileConfig, LfsInfo};

// Test helpers for integration tests (bypass, traverse isolation).
#[doc(hidden)]
pub use crate::dir::traverse::TraverseTestOut;
#[doc(hidden)]
pub use crate::fs::format::{
    test_format_minimal_superblock, test_traverse_filter_gets_superblock_after_push,
    test_traverse_format_attrs,
};
pub use crate::lfs_info::LfsFsinfo;
use crate::lfs_type::OpenFlags;
#[doc(hidden)]
pub use crate::types::LFS_DISK_VERSION;

// Internal APIs exposed for evil/corruption tests that need to manipulate
// metadata directly (lfs_init, lfs_dir_fetch, lfs_dir_commit, etc.).
#[doc(hidden)]
pub use crate::block_alloc::alloc::lfs_alloc_ckpoint;
#[doc(hidden)]
pub use crate::dir::LfsMdir;
#[doc(hidden)]
pub use crate::dir::commit::{lfs_dir_alloc, lfs_dir_commit};
#[doc(hidden)]
pub use crate::dir::fetch::lfs_dir_fetch;
#[doc(hidden)]
pub use crate::dir::traverse::lfs_dir_get;
#[doc(hidden)]
pub use crate::file::lfs_ctz::LfsCtz;
#[doc(hidden)]
pub use crate::file::lfs_ctz::lfs_ctz_fromle32;
#[doc(hidden)]
pub use crate::fs::init::{lfs_deinit, lfs_init};
#[doc(hidden)]
pub use crate::fs::superblock::lfs_fs_prepmove;
#[doc(hidden)]
pub use crate::lfs_superblock::{LfsSuperblock, lfs_superblock_tole32};
#[doc(hidden)]
pub use crate::tag::{lfs_mattr, lfs_mktag};
use crate::types::{lfs_block_t, lfs_off_t, lfs_size_t, lfs_soff_t};
#[doc(hidden)]
pub use crate::util::{lfs_pair_fromle32, lfs_pair_tole32};

/// Format a block device with littlefs.
/// Per lfs.h lfs_format. Calls lfs_format_ (lfs.c:4391).
#[inline]
pub async fn lfs_format<S: Storage>(lfs: &mut Lfs<S>, config: &LfsConfig<S>) -> Result<(), Error> {
    crate::lfs_trace!("lfs_format({:p}, {:p})", lfs, config);
    let err = crate::fs::lfs_format_(lfs, config).await;
    crate::lfs_trace!("lfs_format -> {:?}", err);
    err
}

/// Mount a littlefs.
/// Per lfs.h lfs_mount. Calls lfs_mount_ (lfs.c:4482).
#[inline]
pub async fn lfs_mount<S: Storage>(lfs: &mut Lfs<S>, config: &LfsConfig<S>) -> Result<(), Error> {
    crate::lfs_trace!("lfs_mount({:p}, {:p})", lfs, config);
    crate::fs::lfs_mount_(lfs, config).await
}

/// Unmount a littlefs.
/// Per lfs.h lfs_unmount. Calls lfs_unmount_ (lfs.c:4647).
#[inline]
pub fn lfs_unmount<S>(lfs: &mut Lfs<S>) -> Result<(), Error> {
    crate::fs::lfs_unmount_(lfs)
}

/// Remove a file or directory. Per lfs.h lfs_remove (lfs.c:6193-6195).
#[inline]
pub async fn lfs_remove<S: Storage>(lfs: &mut Lfs<S>, path: &str) -> Result<(), Error> {
    crate::fs::remove::lfs_remove_(lfs, path).await
}

/// Rename or move a file or directory. Per lfs.h lfs_rename (lfs.c:6227-6231).
#[inline]
pub async fn lfs_rename<S: Storage>(
    lfs: &mut Lfs<S>,
    oldpath: &str,
    newpath: &str,
) -> Result<(), Error> {
    crate::fs::rename::lfs_rename_(lfs, oldpath, newpath).await
}

/// Find info about a file or directory. Per lfs.h lfs_stat (lfs.c:6263-6267).
#[inline]
pub async fn lfs_stat<S: Storage>(
    lfs: &mut Lfs<S>,
    path: &str,
    info: &mut LfsInfo,
) -> Result<(), Error> {
    crate::fs::stat::lfs_stat_(lfs, path, info).await
}

/// Get a custom attribute. Per lfs.h lfs_getattr (lfs.c:6090-6105).
#[inline]
pub async fn lfs_getattr<S: Storage>(
    lfs: &mut Lfs<S>,
    path: &str,
    r#type: u8,
    buffer: &mut [u8],
) -> Result<lfs_size_t, Error> {
    crate::fs::attr::lfs_getattr_(lfs, path, r#type, buffer).await
}

/// Set custom attributes. Per lfs.h lfs_setattr (lfs.c:6471-6475).
#[inline]
pub async fn lfs_setattr<S: Storage>(
    lfs: &mut Lfs<S>,
    path: &str,
    r#type: u8,
    buffer: &[u8],
    size: usize,
) -> Result<(), Error> {
    crate::fs::attr::lfs_setattr_(lfs, path, r#type, buffer, size).await
}

/// Remove a custom attribute. Per lfs.h lfs_removeattr (lfs.c:6487-6491).
#[inline]
pub async fn lfs_removeattr<S: Storage>(
    lfs: &mut Lfs<S>,
    path: &str,
    r#type: u8,
) -> Result<(), Error> {
    crate::fs::attr::lfs_removeattr_(lfs, path, r#type).await
}

/// Open a file. Per lfs.h lfs_file_open (lfs.c:6140-6146).
#[inline]
pub async fn lfs_file_open<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
    path: &str,
    flags: OpenFlags,
) -> Result<(), Error> {
    crate::file::ops::lfs_file_open_(lfs, file, path, flags).await
}

/// Open a file with extra configuration. Per lfs.h lfs_file_opencfg (lfs.c:6193-6197).
#[inline]
pub async fn lfs_file_opencfg<'a: 'b, 'b, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'b>,
    path: &str,
    flags: OpenFlags,
    config: &mut LfsFileConfig<'a>,
) -> Result<(), Error> {
    crate::file::ops::lfs_file_opencfg_(lfs, file, path, flags, config).await
}

/// Close a file. Per lfs.h lfs_file_close (lfs.c:6227-6231).
#[inline]
pub async fn lfs_file_close<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
) -> Result<(), Error> {
    crate::file::ops::lfs_file_close_(lfs, file).await
}

/// Synchronize a file on storage. Per lfs.h lfs_file_sync (lfs.c:6263-6267).
#[inline]
pub async fn lfs_file_sync<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
) -> Result<(), Error> {
    crate::file::ops::lfs_file_sync_(lfs, file).await
}

/// Read data from file. Per lfs.h lfs_file_read (lfs.c:6210-6224).
#[inline]
pub async fn lfs_file_read<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
    buffer: &mut [u8],
) -> Result<crate::types::lfs_size_t, Error> {
    crate::file::ops::lfs_file_read_(lfs, file, buffer).await
}

/// Write data to file. Per lfs.h lfs_file_write (lfs.c:6228-6242).
#[inline]
pub async fn lfs_file_write<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
    buffer: &[u8],
) -> Result<crate::types::lfs_size_t, Error> {
    crate::file::ops::lfs_file_write_(lfs, file, buffer).await
}

/// Change the position of the file. Per lfs.h lfs_file_seek (lfs.c:6246-6260).
#[inline]
pub async fn lfs_file_seek<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
    off: lfs_soff_t,
    whence: i32,
) -> Result<crate::types::lfs_off_t, Error> {
    crate::file::ops::lfs_file_seek_(lfs, file, off, whence).await
}

/// Truncate the size of the file. Per lfs.h lfs_file_truncate (lfs.c:6471-6475).
#[inline]
pub async fn lfs_file_truncate<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
    size: lfs_off_t,
) -> Result<(), Error> {
    crate::file::ops::lfs_file_truncate_(lfs, file, size).await
}

/// Return the position of the file. Per lfs.h lfs_file_tell.
#[inline]
pub fn lfs_file_tell<S>(_lfs: &mut Lfs<S>, file: &LfsFile) -> lfs_off_t {
    crate::file::ops::lfs_file_tell_(core::ptr::null(), file)
}

/// Change the position to the beginning of the file. Per lfs.h lfs_file_rewind (lfs.c:6487-6491).
#[inline]
pub async fn lfs_file_rewind<'a, S: Storage>(
    lfs: &mut Lfs<S>,
    file: &mut LfsFile<'a>,
) -> Result<(), Error> {
    crate::file::ops::lfs_file_rewind_(lfs, file).await
}

/// Return the size of the file. Per lfs.h lfs_file_size (lfs.c:6495-6499).
#[inline]
pub fn lfs_file_size<S>(lfs: &mut Lfs<S>, file: &LfsFile) -> lfs_off_t {
    crate::file::ops::lfs_file_size_(lfs, file)
}

/// Create a directory. Per lfs.h lfs_mkdir (lfs.c:6503-6507).
#[inline]
pub async fn lfs_mkdir<S: Storage>(lfs: &mut Lfs<S>, path: &str) -> Result<(), Error> {
    crate::fs::mkdir::lfs_mkdir_(lfs, path).await
}

/// Open a directory. Per lfs.h lfs_dir_open (lfs.c:6511-6515).
#[inline]
pub async fn lfs_dir_open<S: Storage>(
    lfs: &mut Lfs<S>,
    dir: &mut LfsDir,
    path: &str,
) -> Result<(), Error> {
    crate::dir::open::lfs_dir_open_(lfs, dir, path).await
}

/// Close a directory. Per lfs.h lfs_dir_close.
#[inline]
pub fn lfs_dir_close<S>(lfs: &mut Lfs<S>, dir: &mut LfsDir) -> Result<(), Error> {
    crate::dir::open::lfs_dir_close_(lfs, dir)
}

/// Read an entry in the directory. Per lfs.h lfs_dir_read.
#[inline]
pub async fn lfs_dir_read<S: Storage>(
    lfs: &mut Lfs<S>,
    dir: &mut LfsDir,
    info: &mut LfsInfo,
) -> Result<bool, Error> {
    crate::dir::open::lfs_dir_read_(lfs, dir, info).await
}

/// Change the position of the directory. Per lfs.h lfs_dir_seek.
#[inline]
pub async fn lfs_dir_seek<S: Storage>(
    lfs: &mut Lfs<S>,
    dir: &mut LfsDir,
    off: lfs_off_t,
) -> Result<(), Error> {
    crate::dir::open::lfs_dir_seek_(lfs, dir, off).await
}

/// Return the position of the directory. Per lfs.h lfs_dir_tell (lfs.c:6400-6412).
#[inline]
pub fn lfs_dir_tell<S>(lfs: &mut Lfs<S>, dir: &mut LfsDir) -> lfs_soff_t {
    crate::dir::open::lfs_dir_tell_(lfs, dir)
}

/// Change the position to the beginning of the directory. Per lfs.h lfs_dir_rewind.
#[inline]
pub async fn lfs_dir_rewind<S: Storage>(lfs: &mut Lfs<S>, dir: &mut LfsDir) -> Result<(), Error> {
    crate::dir::open::lfs_dir_rewind_(lfs, dir).await
}

/// Find on-disk info about the filesystem. Per lfs.h lfs_fs_stat (lfs.c:6449-6453).
#[inline]
pub async fn lfs_fs_stat<S: Storage>(
    lfs: &mut Lfs<S>,
    fsinfo: &mut LfsFsinfo,
) -> Result<(), Error> {
    crate::fs::lfs_fs_stat_(lfs, fsinfo).await
}

/// Find the current size of the filesystem. Per lfs.h lfs_fs_size (lfs.c:6449-6453).
#[inline]
pub async fn lfs_fs_size<S: Storage>(lfs: &mut Lfs<S>) -> Result<lfs_size_t, Error> {
    crate::fs::stat::lfs_fs_size_(lfs).await
}

/// Callback type for lfs_fs_traverse. Per lfs.h int (*cb)(void*, lfs_block_t).
pub type LfsTraverseCb = dyn FnMut(lfs_block_t) -> Result<(), Error>;

/// Traverse through all blocks in use by the filesystem. Per lfs.h lfs_fs_traverse.
#[inline]
pub async fn lfs_fs_traverse<S: Storage>(
    lfs: &mut Lfs<S>,
    cb: &mut LfsTraverseCb,
) -> Result<(), Error> {
    crate::fs::traverse::lfs_fs_traverse_(lfs, cb, false).await
}

/// Attempt to make the filesystem consistent. Per lfs.h lfs_fs_mkconsistent (lfs.c:6479-6483).
#[inline]
pub async fn lfs_fs_mkconsistent<S: Storage>(lfs: &mut Lfs<S>) -> Result<(), Error> {
    crate::fs::consistent::lfs_fs_mkconsistent_(lfs).await
}

/// Attempt any janitorial work. Per lfs.h lfs_fs_gc (lfs.c:6495-6499).
#[inline]
pub async fn lfs_fs_gc<S: Storage>(lfs: &mut Lfs<S>) -> Result<(), Error> {
    crate::fs::consistent::lfs_fs_gc_(lfs).await
}

/// Force consistency (deorphan, demove, desuperblock). For testing.
#[doc(hidden)]
pub async fn lfs_fs_forceconsistency<S: Storage>(lfs: &mut Lfs<S>) -> Result<(), Error> {
    crate::fs::superblock::lfs_fs_forceconsistency(lfs).await
}

/// Prepend orphan count delta to gstate. For testing power-loss paths.
#[doc(hidden)]
pub fn lfs_fs_preporphans<S>(lfs: &mut Lfs<S>, orphans: i8) -> Result<(), Error> {
    crate::fs::superblock::lfs_fs_preporphans(lfs, orphans)
}

/// True if gstate has pending orphans. For testing.
#[doc(hidden)]
pub fn lfs_fs_hasorphans<S>(lfs: &Lfs<S>) -> bool {
    crate::lfs_gstate::lfs_gstate_hasorphans(&lfs.gstate)
}

/// Grow (or shrink) the filesystem to a new size. Per lfs.h lfs_fs_grow (lfs.c:6511-6515).
#[inline]
pub async fn lfs_fs_grow<S: Storage>(
    lfs: &mut Lfs<S>,
    block_count: lfs_size_t,
) -> Result<(), Error> {
    crate::fs::grow::lfs_fs_grow_(lfs, block_count).await
}
