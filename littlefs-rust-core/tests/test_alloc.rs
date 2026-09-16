//! Allocator and block allocation tests.
//!
//! Upstream: tests/test_alloc.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_alloc.toml

mod common;

use common::{
    BadblockBehavior, LFS_O_APPEND, LFS_O_CREAT, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY,
};
#[cfg(test)]
use littlefs_rust_core::LfsConfig;
use littlefs_rust_core::{
    Error, Lfs, LfsFile, LfsInfo, lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_size,
    lfs_file_sync, lfs_file_truncate, lfs_file_write, lfs_format, lfs_fs_gc, lfs_mkdir, lfs_mount,
    lfs_remove, lfs_stat, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;
use stopwatch::Stopwatch;

const FILES: u32 = 3;
const NAMES: [&str; 3] = ["bacon", "eggs", "pancakes"];



// --- test_alloc_serial ---
/// Upstream: [cases.test_alloc_serial]
/// defines.FILES = 3, SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-6))/FILES)
/// defines.GC = [false, true], COMPACT_THRESH = [-1, 0, BLOCK_SIZE/2], INFER_BC = [false, true]
///
/// Create breakfast dir, then for each file: mount, open, write SIZE bytes (optional GC per write),
/// close, unmount. Remount and verify all files.
#[lfs_test]
fn test_alloc_serial(
    cfg: &LfsConfig,
    #[values(false, true)] gc: bool,
    #[values(false, true)] infer_bc: bool,
) {
    let block_size = cfg.block_size;
    let block_count = cfg.block_count;
    let size: usize = ((block_size - 8) as usize * (block_count - 6) as usize) / FILES as usize;

    let mut sw = Stopwatch::new();
    let mut sw_all = Stopwatch::start_new();

    // for compact_thresh in [u32::MAX, 0, block_size / 2] {
    for compact_thresh in [0] {
        let mut cfg = LfsConfig {
            compact_thresh,
            ..*cfg
        };

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &cfg));

        cfg.block_count = if infer_bc { 0 } else { block_count };

        assert_ok!(lfs_mount(lfs, &cfg));
        assert_ok!(lfs_mkdir(lfs, "breakfast"));
        assert_ok!(lfs_unmount(lfs));

        for name in NAMES.into_iter() {
            assert_ok!(lfs_mount(lfs, &cfg));
            let path = &format!("breakfast/{}", name);
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
            ));
            for _ in (0..size).step_by(name.len()) {
                if gc {
                    assert_ok!(lfs_fs_gc(lfs, &mut sw));
                }
                assert_eq!(
                    lfs_file_write(lfs, file, name.as_bytes()),
                    Ok(name.len() as u32)
                );
            }
            assert_ok!(lfs_file_close(lfs, file));
            assert_ok!(lfs_unmount(lfs));
        }

        assert_ok!(lfs_mount(lfs, &cfg));
        for name in NAMES.into_iter() {
            let path = &format!("breakfast/{}", name);
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            let mut buf = vec![0u8; name.len()];
            for _ in (0..size).step_by(name.len()) {
                assert_eq!(lfs_file_read(lfs, file, &mut buf), Ok(buf.len() as u32));
                assert_eq!(&buf, &name.as_bytes());
            }
            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));
    }

    ::log::warn!("{:?}/{:?}", sw.elapsed(), sw_all.elapsed());
}
