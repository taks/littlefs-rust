//! Entry/inline file corner case tests.
//!
//! Upstream: tests/test_entries.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_entries.toml
//!
//! Metadata spill (4 files × 200B inline) and directory compaction.

mod common;

use common::{LFS_O_CREAT, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY};
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsFile, lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write,
    lfs_format, lfs_mount, lfs_remove, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;

// --- test_entries_grow ---
#[lfs_test]
fn test_entries_grow(cfg: &LfsConfig, #[values(512)] cache_size: u32) {
    assert_eq!(cfg.cache_size, 512);
    if !cfg.cache_size.is_multiple_of(cfg.prog_size) {
        return;
    }

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let buf = [b'c'; 1024];
    for i in 0..4 {
        let path = &format!("hi{i}");
        let size = 20usize;
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok!(lfs_file_close(lfs, file));
    }

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, "hi1", LFS_O_RDONLY));
    let mut rb = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut rb[..20]);
    assert_eq!(n, Ok(20));
    assert_ok!(lfs_file_close(lfs, file));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..200]);
    assert_eq!(n, Ok(200));
    assert_ok!(lfs_file_close(lfs, file));

    for i in 0..4 {
        let path = &format!("hi{i}");
        let size = if i == 1 { 200 } else { 20 };
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_eq!(&rb[..size], &buf[..size]);
        assert_ok!(lfs_file_close(lfs, file));
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_entries_shrink ---
#[lfs_test]
fn test_entries_shrink(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let buf = [b'c'; 1024];
    for i in 0..4 {
        let path = &format!("hi{i}");
        let size = if i == 1 { 200 } else { 20 };
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok!(lfs_file_close(lfs, file));
    }

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, "hi1", LFS_O_RDONLY));
    let mut rb = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut rb[..200]);
    assert_eq!(n, Ok(200));
    assert_ok!(lfs_file_close(lfs, file));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..20]);
    assert_eq!(n, Ok(20));
    assert_ok!(lfs_file_close(lfs, file));

    for i in 0..4 {
        let path = &format!("hi{i}");
        let size = 20;
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_eq!(&rb[..size], &buf[..size]);
        assert_ok!(lfs_file_close(lfs, file));
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_entries_spill ---
#[lfs_test]
fn test_entries_spill(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let buf = [b'c'; 256];
    for i in 0..4 {
        let path = &format!("hi{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..200]);
        assert_eq!(n, Ok(200));
        assert_ok!(lfs_file_close(lfs, file));
    }

    let mut rb = [0u8; 256];
    for i in 0..4 {
        let path = &format!("hi{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..200]);
        assert_eq!(n, Ok(200));
        assert_eq!(&rb[..200], &buf[..200]);
        assert_ok!(lfs_file_close(lfs, file));
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_entries_push_spill ---
#[lfs_test]
fn test_entries_push_spill(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let buf = [b'c'; 256];
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hi0",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..200]);
    assert_eq!(n, Ok(200));
    assert_ok!(lfs_file_close(lfs, file));

    for i in 1..4 {
        let path = &format!("hi{i}");
        let size = if i == 1 { 20 } else { 200 };
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok!(lfs_file_close(lfs, file));
    }

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, "hi1", LFS_O_RDONLY));
    let mut rb = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut rb[..20]);
    assert_eq!(n, Ok(20));
    assert_ok!(lfs_file_close(lfs, file));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..200]);
    assert_eq!(n, Ok(200));
    assert_ok!(lfs_file_close(lfs, file));

    for i in 0..4 {
        let path = &format!("hi{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..200]);
        assert_eq!(n, Ok(200));
        assert_ok!(lfs_file_close(lfs, file));
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_entries_drop ---
#[lfs_test]
fn test_entries_drop(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let buf = [b'c'; 256];
    for i in 0..4 {
        let path = &format!("hi{i}");
        let size = if i == 1 { 200 } else { 20 };
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok!(lfs_file_close(lfs, file));
    }

    assert_ok!(lfs_remove(lfs, "hi1"));
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..20]);
    assert_eq!(n, Ok(20));
    assert_ok!(lfs_file_close(lfs, file));

    let mut rb = [0u8; 256];
    for i in 0..4 {
        let path = &format!("hi{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..20]);
        assert_eq!(n, Ok(20));
        assert_ok!(lfs_file_close(lfs, file));
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_entries_create_too_big ---
// Upstream: [cases.test_entries_create_too_big]
#[lfs_test]
fn test_entries_create_too_big(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let path = &"m".repeat(200);
    let size = 400usize;
    let wbuf = [b'c'; 1024];
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &wbuf[..size]);
    assert_eq!(n, Ok(size as u32));
    assert_ok!(lfs_file_close(lfs, file));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    let mut rbuf = [0u8; 1024];
    let n = lfs_file_read(lfs, file, &mut rbuf[..size]);
    assert_eq!(n, Ok(size as u32));
    assert_eq!(&rbuf[..size], &wbuf[..size]);
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_unmount(lfs));
}

// --- test_entries_resize_too_big ---
// Upstream: [cases.test_entries_resize_too_big]
// 200-byte path needs ample blocks; 2048 matches upstream geometry (ERASE_COUNT=1M/512).
#[lfs_test]
fn test_entries_resize_too_big(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let path = &"m".repeat(200);
    let wbuf = [b'c'; 1024];
    let mut rbuf = [0u8; 1024];

    // Create with 40 bytes
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &wbuf[..40]);
    assert_eq!(n, Ok(40));
    assert_ok!(lfs_file_close(lfs, file));

    // Read 40 bytes
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    let n = lfs_file_read(lfs, file, &mut rbuf[..40]);
    assert_eq!(n, Ok(40));
    assert_eq!(&rbuf[..40], &wbuf[..40]);
    assert_ok!(lfs_file_close(lfs, file));

    // Truncate and write 400 bytes
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &wbuf[..400]);
    assert_eq!(n, Ok(400));
    assert_ok!(lfs_file_close(lfs, file));

    // Read 400 bytes
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    let n = lfs_file_read(lfs, file, &mut rbuf[..400]);
    assert_eq!(n, Ok(400));
    assert_eq!(&rbuf[..400], &wbuf[..400]);
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_unmount(lfs));
}
