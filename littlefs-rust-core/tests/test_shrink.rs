//! Upstream: tests/test_shrink.toml
//!
//! Shrink/grow block count tests via lfs_fs_grow (shrink path).

mod common;

use common::{LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY, LfsConfig};
use littlefs_rust_core::{
    Error, Lfs, LfsFile, lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write, lfs_format,
    lfs_fs_grow, lfs_mount, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;

/// Upstream: [cases.test_shrink_simple]
///
/// defines.BLOCK_COUNT = [10, 15, 20]
/// defines.AFTER_BLOCK_COUNT = [5, 10, 15, 19]
/// if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT'
///
/// Format on BLOCK_COUNT blocks, shrink via lfs_fs_grow(AFTER_BLOCK_COUNT).
/// If sizes differ, mount with original config fails (LFS_ERR_INVAL),
/// mount with reduced config succeeds.
#[lfs_test]
#[tokio::test]
async fn test_shrink_simple<'a>(
    cfg: &LfsConfig<'a>,
    #[values(10, 15, 20)] block_count: u32,
    #[values(5, 10, 15, 19)] after_block_count: u32,
) {
    if after_block_count > cfg.block_count {
        return;
    }

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg).await);
    assert_ok!(lfs_mount(lfs, cfg).await);
    assert_ok!(lfs_fs_grow(lfs, after_block_count).await);
    let _ = lfs_unmount(lfs);

    if cfg.block_count != after_block_count {
        assert_err!(Error::Invalid, lfs_mount(lfs, cfg).await);
    }

    // Mount with reduced config
    let cfg2 = LfsConfig {
        block_count: after_block_count,
        ..*cfg
    };
    let lfs2 = &mut Lfs::default();
    assert_ok!(lfs_mount(lfs2, &cfg2).await);
    assert_ok!(lfs_unmount(lfs2));
}

/// Upstream: [cases.test_shrink_full]
///
/// defines.BLOCK_COUNT = [10, 15, 20]
/// defines.AFTER_BLOCK_COUNT = [5, 7, 10, 12, 15, 17, 20]
/// defines.FILES_COUNT = [7, 8, 9, 10]
/// if = 'AFTER_BLOCK_COUNT <= BLOCK_COUNT && FILES_COUNT + 2 < BLOCK_COUNT'
///
/// Create FILES_COUNT+1 files of BLOCK_SIZE-0x40 bytes. Shrink via
/// lfs_fs_grow(AFTER_BLOCK_COUNT). On success: verify all files and
/// remount with reduced config. On LFS_ERR_NOTEMPTY: shrink expected
/// to fail (too many files for smaller device).
#[lfs_test]
#[tokio::test]
async fn test_shrink_full<'a>(
    cfg: &LfsConfig<'a>,
    #[values(10, 15, 20)] block_count: u32,
    #[values(5, 7, 10, 12, 15, 17, 20)] after_block_count: u32,
    #[values(7, 8, 9, 10)] files_count: u32,
) {
    if after_block_count > cfg.block_count || files_count + 2 >= cfg.block_count {
        return;
    }

    let size = cfg.block_size - 0x40;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg).await);
    assert_ok!(lfs_mount(lfs, cfg).await);

    // Create FILES_COUNT+1 files of BLOCK_SIZE - 0x40 bytes
    for i in 0..files_count + 1 {
        let path = format!("file_{:03}", i);
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, &path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,).await);

        let mut wbuffer = vec![b'b'; cfg.block_size as usize];
        let header = format!("Hi {:03}", i);
        wbuffer[..header.len()].copy_from_slice(header.as_bytes());

        let n = lfs_file_write(lfs, file, &wbuffer[..size as usize]).await;
        assert_eq!(n, Ok(size));
        assert_ok!(lfs_file_close(lfs, file).await);
    }

    let err = lfs_fs_grow(lfs, after_block_count).await;
    if err.is_ok() {
        // Verify all files while still mounted
        for i in 0..files_count + 1 {
            let path = format!("file_{:03}", i);
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs, file, &path, LFS_O_RDONLY).await);

            let mut rbuffer = vec![0u8; size as usize];
            let n = lfs_file_read(lfs, file, &mut rbuffer).await;
            assert_eq!(n, Ok(size));
            assert_ok!(lfs_file_close(lfs, file).await);

            // Build reference buffer
            let mut wbuffer_ref = vec![b'b'; size as usize];
            let header = format!("Hi {:03}", i);
            wbuffer_ref[..header.len()].copy_from_slice(header.as_bytes());
            assert_eq!(rbuffer, wbuffer_ref);
        }
    } else {
        assert_eq!(err, Err(Error::NotEmpty));
    }

    assert_ok!(lfs_unmount(lfs));

    if err.is_ok() {
        if after_block_count != cfg.block_count {
            assert_err!(Error::Invalid, lfs_mount(lfs, cfg).await);
        }

        // Remount with reduced config and verify files again
        let cfg2 = LfsConfig {
            block_count: after_block_count,
            ..*cfg
        };
        let lfs2 = &mut Lfs::default();
        assert_ok!(lfs_mount(lfs2, &cfg2).await);

        for i in 0..files_count + 1 {
            let path = format!("file_{:03}", i);
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs2, file, &path, LFS_O_RDONLY).await);

            let mut rbuffer = vec![0u8; size as usize];
            let n = lfs_file_read(lfs2, file, &mut rbuffer).await;
            assert_eq!(n, Ok(size));
            assert_ok!(lfs_file_close(lfs2, file).await);

            let mut wbuffer_ref = vec![b'b'; size as usize];
            let header = format!("Hi {:03}", i);
            wbuffer_ref[..header.len()].copy_from_slice(header.as_bytes());
            assert_eq!(rbuffer, wbuffer_ref);
        }

        let _ = lfs_unmount(lfs2);
    }
}
