//! Block device layer. Per lfs.c lfs_bd_*, lfs_cache_*.

#[expect(clippy::module_inception)]
pub(crate) mod bd;
mod lfs_cache;

pub use lfs_cache::LfsCache;
